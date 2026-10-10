# Story 1.24: pwsd-fidelity-relock

Status: ready-for-dev

<!-- Created 2026-07-31 by the 1-13 code-review decision (operator: "doc now + re-lock story").
     This story owns the CODE half of the PWSD-fidelity cluster; the doc half landed in the
     1-13 review pass (deviation recorded, behavior pinned by the FP-C1.6 assert). -->

## Story

As the operator of the Honest Advisor,
I want the Monte-Carlo credibility layer's block-length estimator brought to cited-literature fidelity under a formal anchor re-lock,
so that the robustness gate's null distribution is defensible against the exact references the code cites — with the anchored history migrated honestly, never silently.

## Acceptance Criteria

1. **Window scale**: the flat-top lag window evaluates `λ(k/(2m̂))` per Politis-White 2004 (flat to lag m̂, tapering to 2m̂), replacing the shipped `λ(k/m̂)`; the m̂ search checks `ρ̂(m+1..m+K_N)` per the reference (shipped: `ρ̂(m..)`); the FP-C1.6 pin is re-baselined to the new l_ar1 with the change stated in the assert message.
2. **Auto-L target**: the cross-symbol `|r|`-mean choice is either per-symbol-standardized (or replaced with a per-symbol max/median policy) with an architect sign-off recorded in this story, or explicitly re-ratified as-is — no silent carry-forward. The absolute-ε zero-variance guard becomes scale-relative in the same pass.
3. **Volume proxy**: synthetic bars carry the correct symbol's volume at the correct bar offset (returns k spans bars k→k+1), replacing the symbol-0-at-ret_idx proxy; the slippage-consumption path (`volume_usd`) is traced and the impact (if any) on regenerated bodies is quantified in the re-lock report.
4. **Formal re-lock per ADR-0045 § D6**: the affected MC/sweep reports are regenerated under a NEW anchor namespace; the old `mc-robustness-2026-06` rows stay byte-frozen as history; an errata/decision note records old-vs-new L values per scenario and the verdict deltas (expectation per the review's direction analysis: longer L ⇒ crowns same or fewer ⇒ ship-passive strengthens — any OPPOSITE movement is a REGRESSION-class finding requiring operator escalation before ship).
5. Standing floor: anchors green under the post-re-lock set (old rows intact + new rows added); `spec_lint` PASS; the FROZEN gate (`classify_verdict`/`verdict_bands`/`compute_robustness_flag`/`rank_candidates`) byte-untouched with the identity-test obligation discharged.

## Tasks / Subtasks

- [x] **Architect pass DONE 2026-10-10.** Two scope findings, both from deriving the inventory from
      the criterion rather than trusting this story's own enumeration (`#126`/`#119`'s lesson).

      **FINDING 1 — AC4's namespace scope covers 7 of 35 affected rows.** The criterion is *"every
      anchored row whose body depends on `politis_white_block_length`"*. Derived:
      `BlockLengthPolicy::Auto` is selected at four production sites — `mc_harness.rs:224`,
      `bin/monte_carlo.rs:710`, `bin/param_robustness_sweep.rs:600` (+ `:1109/:1261/:1491`) — and
      all **34** θ-surfaces in `scripts/relock/surfaces.tsv` route through the block-bootstrap-real
      generator, plus the one `-mc` scenario. That is **35 rows**, and they span **four**
      namespaces:

      | namespace | θ-surfaces | + MC | total |
      |---|---|---|---|
      | `mc-robustness-2026-06` | 6 | 1 | **7** ← the only one AC4 names |
      | `horizon-retest-robustness` | 8 | — | 8 |
      | `perp-basis-signal-robustness` | 8 | — | 8 |
      | `perp-basis-mn-spread` | 12 | — | 12 |

      Re-locking only `mc-robustness-2026-06` would leave **28 rows anchored to bodies the code no
      longer produces** — manufacturing exactly the code-vs-evidence drift (`#93`) that story 1-29
      spent 2026-10-01 measuring and closing, in the same week, deliberately. **AC4's scope is
      therefore read as all 35 rows.** This is a correction to this story's own AC, recorded not
      escalated: it widens work rather than changing a claim, and the widening is the conservative
      direction.

      **FINDING 2 — the `|r|` transform is RATIFIED; only the aggregation is not.** I had this down
      as a fourth unlisted deviation and it is not: `bootstrap.rs:313` records it as **rule
      D-C1.4** (*"compute PWSD on the universe-average absolute log-return series"*). What D-C1.4
      states is the rule, not the reason — and the reason is worth writing down, because running the
      selector on `|r|` rather than `r` targets **volatility clustering** instead of raw-return
      dependence, which for drawdown/robustness statistics is the structure that matters and is the
      defensible choice. Unrecorded, it reads like an oversight; AC2's "no silent carry-forward"
      covers it.

      **AC2 — the decision, and why it is a decision about METHOD rather than a policy picked from
      theory.** PWSD is **scale-invariant per series**: under `r → c·r` every `γ̂` scales by `c²`, so
      `ĝ²/Ĝ²` is unchanged, `ρ̂ = γ̂(k)/γ̂(0)` is unchanged and the band is scale-free. So
      per-symbol standardisation would not move a per-symbol estimate **at all** — the defect exists
      *only* because averaging across different scales happens **before** estimation, which is what
      lets the highest-vol symbol dominate the mixture. That sharpens the option set:

      - **(a) standardise-then-average** — targets the defect precisely, one PWSD call, but still
        estimates `b` for a synthetic mixture that is not any resampled series.
      - **(b) estimate-per-symbol-then-reduce** (max or median) — every PWSD call is applied to a
        series that actually *is* resampled, which is the literature's premise; only the reduction
        is policy, and it is stated. One L remains structural: the stationary-bootstrap index is
        drawn **once** and shared across symbols (ADR-0051 § D1), so per-symbol L is unavailable
        without breaking the cross-sectional dependence structure the MN/cross-sectional family
        rests on.
      - **(c) re-ratify as-is** — permitted by AC2, and the weakest of the three unless the
        measurement says otherwise.

      **(b)-with-max is the leading candidate** — most literature-faithful, and conservative: if any
      symbol carries dependence at lag L, a shorter block destroys it. **But the direction is NOT
      assumed.** Averaging `|r|` across symbols suppresses idiosyncratic noise and can *raise* the
      measured persistence of the common volatility factor, so the shipped mean-series L may well be
      **longer** than any per-symbol L — in which case (b)-max *shortens* L, which drives AC4's
      guard the wrong way (shorter L ⇒ more crowns ⇒ thesis weakens). Deciding (a) vs (b) from
      theory would be guessing at the one number this story exists to measure.

      **So the method, pre-registered before any number is seen:** compute L offline for all four
      candidates on each of the 35 scenarios' own input series, publish the table, then adopt the
      most literature-faithful candidate **unless** it moves the direction guard adversely — in
      which case escalate under AC4 rather than quietly picking the convenient policy. Writing the
      criterion down first is what stops the choice being post-hoc; it is the same pre-registration
      discipline the robustness gate itself uses.

      **Re-lock plan.**
      - **Namespace**: `mc-robustness-2026-10` (dated, matching the `mc-robustness-2026-06`
        convention it supersedes). All four old namespaces stay byte-frozen as history.
      - **Ordering — measure L BEFORE regenerating.** The L table is ~seconds of offline compute
        per scenario (PWSD is a pure function of the input returns and needs no backtest); the
        regeneration is **≈ 75–80 min** (measured 2026-09-29: 32 cheap θ-surfaces ≈ 11 min, the two
        expensive ones ≈ 62 min, at `RAYON_NUM_THREADS=8`). So the direction guard can be checked
        for the price of seconds, and an adverse direction escalates **before** 80 minutes is spent
        producing bodies that must not ship. This inverts the task order below deliberately.
      - **Apparatus to extend, not re-implement** (`#133`/`#136`): `scripts/relock/surfaces.tsv` is
        the derived manifest and `relock_manifest.rs` gate-tests it; `theta_surface_reproduction.rs`
        already re-runs all 34 plus the MC row and compares against the anchors, so the post-re-lock
        verification is a gate that exists rather than one to be written.
      - **AC5's identity-test obligation**: the FROZEN gate calls this function, so the `usize`
        return signature is frozen and the identity test must show ranking is unchanged *given the
        same L* — i.e. it tests the gate, not the estimator.
- [~] **Dev — AC1 parameterised and MEASURABLE 2026-10-10, shipped behaviour byte-identical.**
      `PwsdVariant::{Shipped, Literature}` names the two conventions and
      `politis_white_block_length_variant` evaluates either; the public
      `politis_white_block_length` delegates to `Shipped`, so its result is unchanged and all 35
      anchored bodies plus the FP-C1.6 pin stay valid **while the fix is being measured**. One
      implementation, two scalars (`window_denominator_factor`, `search_offset`) — the alternative
      was a second copy, which is what `#133`/`#136` were filed for.
      - `shipped_variant_is_byte_identical_to_the_public_fn` pins the refactor across 20 inputs
        (empty, single, zero-variance, deterministic cycle, and AR(1) at φ ∈ {0, .3, .6, .9, −.5}
        × N ∈ {32, 251, 1024}) rather than on the one fixture FP-C1.6 happens to use — otherwise
        "behaviour preserved" would rest on a single sample.

      **FIRST MEASUREMENT, and it undercuts the premise of AC1's direction claim.** On the FP-C1.6
      fixtures the two conventions **agree**: AR(1) φ=0.6 → L=7 *both* ways; i.i.d. → L=1 both ways.
      The two deviations partially **cancel**. At N=500 the band is `2·√(log₁₀500/500) = 0.147`, and
      `ρ(k)=0.6^k` enters it at k=4, so the shipped search (`ρ̂(m..m+K_N−1)`) settles at **m̂=4**
      while the literature search (`ρ̂(m+1..m+K_N)`) settles at **m̂=3** — and the literature's wider
      window on the smaller m̂ (effective `|k| ≤ 2m̂ = 6`) lands on the same `b̂` after rounding as
      the shipped narrow window on the larger one (`|k| ≤ m̂ = 4`).

      So the module doc's *"auto-L comes out systematically SHORT"* — and AC1's and AC4's
      expectation built on it — is an algebraic reading that **this fixture does not demonstrate**.
      It is not refuted either; one fixture at one N and one φ settles nothing. What it does
      establish is that the direction is genuinely open per scenario, and that the pre-registered
      measure-first ordering was not ceremony: had the fix been landed on the strength of the
      "systematically short" reading, the 80-minute regeneration would have been spent before
      anyone checked whether the premise held.
      - `literature_variant_direction_on_the_fp_c1_6_fixtures` records both numbers and asserts
        only that the Literature arm still detects serial dependence (AR(1) L > i.i.d. L) — a
        wiring check, not a direction claim, because asserting a direction here would pre-judge
        what the 35-scenario measurement is for.
      **AC2's ε half and AC3 DONE 2026-10-10, and both are MEASURED anchor-neutral.**

      - **AC2 ε — and module-doc deviation #3 UNDERCOUNTED: there are TWO absolute-ε tests, not
        one.** The doc names the `m̂` guard (`ρ̂(0) < f64::EPSILON`); there is a second at the `b̂`
        degenerate guard (`Ĝ.abs() < f64::EPSILON`). Both now compare on matching units — the
        standard deviation against `scale`, and `Ĝ` against `scale²`, where
        `scale = max|rₜ|` — so "small" stops being read as "constant".
        **Found by the test, on my own fix**: `scale_relative_guard_differs_only_for_tiny_variance_series`
        measured a ±1e-9 series returning L=1 while its 1e9-scaled twin returned L=68, so fixing
        only the first site left the estimator magnitude-sensitive in exactly the way the test
        existed to catch. `g_slope == 0.0` stays exact: zero slope scales to zero slope, so it is
        a true degeneracy rather than a tolerance.
      - **AC3 volume — two defects in one expression.** `source_bars[0].1.get(ret_idx)` took
        symbol 0's volume for *every* symbol (BTC's volume stamped onto XRP's bars) and read the
        return's **origin** bar, when return k spans bars k→k+1 and the bar being built is the
        destination. Now `source_bars[sym_i].1.get(ret_idx + 1)`; `sym_i` was already in scope and
        `returns_by_sym[sym_i]` two dozen lines up already indexed per-symbol — only the volume
        reached for `[0]`.
      - **AC3's required trace, and the old comment's claim survives but NOT as stated.**
        `volume_usd` — the only slippage input that could consume it — comes from
        `cfg.volume_usd_per_symbol`, a **config map**, never from `Bar::volume`; and it is read
        only by the `SquareRoot` arm. But `Bar::volume` *is* read in three places: the resampler
        sums it (`resample.rs:316`), the TCN/PatchTST overlays feature `ln(1+volume)`, and the
        ADR-0071 **OBV** node accumulates it (`composed/node.rs:419`). None is on the 35
        scenarios' path — they are cross-sectional sweeps, and OBV reaches only the engine's
        `v0.obv` arm and the agent runtime. So "volume is not load-bearing" is true *for these
        scenarios* and false as a general statement; it becomes body-affecting the moment an OBV
        or overlay surface is run over synthetic bars.
      - **NEUTRALITY MEASURED, not argued.** `v1-carry-horizon-daily-theta-surface-2024-block-bootstrap-real-fy`
        re-run with both fixes in reproduces **`0427481b54bc…` — byte-identical to its anchor**,
        with L=3 unchanged. That surface is the strongest available case for the volume half: the
        daily horizon is the one that exercises the volume-summing resampler, so if resampled
        volume reached anything downstream, this is where it would have shown. `ANCHORS PASS
        (119/119)`.
        **Scope of that claim, stated rather than glossed: one surface, not 35.** It establishes
        the mechanism is neutral; the family-wide confirmation comes with the regeneration.
      - **Consequence for the re-lock — a clean attribution.** Because these two fixes move no
        body, any delta the regeneration produces is attributable **solely** to AC1's convention
        change plus AC2's aggregation policy. That is what lets the re-lock report say *what*
        moved the numbers instead of *that* they moved.
      - Remaining in this task: flip the default to `PwsdVariant::Literature` and re-baseline the
        FP-C1.6 pin — **after** the 35-scenario L table, per the pre-registered ordering.
- [x] **THE L TABLE — measured 2026-10-10, all 34 surfaces, seconds not hours.**
      `param_robustness_sweep --report-block-length-only` reports the candidate table from the
      **same** `path_gen` and corpus the real run uses, then exits before any backtest. It
      self-checks: `shipped.mean_abs` must equal the generator's own `selected_block_length`, and
      it **agreed on 34/34** — so the diagnostic is measuring the series the run resamples, which
      is why it was factored out of `generate()` rather than copied.

      **1. AC1's direction claim is CONFIRMED on real corpora: 32 longer, 0 shorter, 2 identical.**
      The module doc's *"systematically SHORT"* reading holds — and the FP-C1.6 toy fixture, where
      the two conventions agreed, was the exception rather than the rule. Good thing the fixture
      was not generalised from.

      | corpus shape | n | shipped L | literature L | clamp |
      |---|---|---|---|---|
      | 2023 full-year 1h | 14 | 204 | **281** | 281 ← **saturated** |
      | 2024 full-year 1h | 12 | 200 | **282** | 282 ← **saturated** |
      | 2023 4h | 2 | 80 | 115 | 141 |
      | 2024 4h | 2 | 49 | 73 | 141 |
      | 2023 daily | 2 | 9 | 14 | 58 |
      | 2024 daily | 2 | 3 | 3 | 58 |

      **2. The hazard flagged BEFORE measuring has materialised: 26 of 34 scenarios SATURATE the
      clamp** under the literature convention. `ceil(min(3·√N, N/3))` is 281 for 2023 hourly and
      282 for 2024, and the literature L lands exactly there. For those 26 the reported
      `selected_block_length_L` stops being an estimate and becomes **the cap**.

      **3. How much work the clamp does — measured, because a clamped value cannot show it.**
      `politis_white_b_hat_unclamped` exposes the pre-clamp `b̂` through the same single pipeline
      (one core, two views, so clamped and unclamped cannot disagree about the estimate). On
      `v1-momentum-theta-surface-2023`: **raw b̂ shipped = 204.118** (clamp 281, *not* binding) and
      **raw b̂ literature = 329.647** (capped to 281, binding by ~17%). So the clamp is a **mild
      trim, not a fabrication** — but it is still the cap that gets reported.

      **4. Consequence for AC2: the policy choice is UNOBSERVABLE on 26 of 34 scenarios.** Where
      the clamp binds, `mean_abs`, `standardized_mean_abs`, `per_symbol_max` and
      `per_symbol_median` all collapse to 281/282. AC2's decision is therefore only observable on
      the **8** non-full-year surfaces, and there the policies differ materially — 2023 4h:
      `mean_abs` 115 · `std_mean_abs` 89 · `per_sym_max` 124 · `per_sym_median` 67.

      **5. Why this is ESCALATED rather than decided.** The pre-registered criterion said: adopt
      the most literature-faithful candidate unless it moves AC4's direction guard adversely, then
      escalate. The direction is *favourable* (longer L ⇒ wider null ⇒ same-or-fewer crowns), so
      the criterion says proceed. But the criterion did not contemplate clamp saturation, and what
      it implies is outside what a criterion about aggregation can settle:

      - adopting Literature changes what the corpus's `selected_block_length_L` column **means**
        for 26 rows — from "what the estimator selected" to "what the cap allowed". That is a
        change in what the evidence claims about its own gate, which is stop-condition territory;
      - the alternative — raising the clamp so the estimate is reported rather than trimmed —
        means removing a **defensive bound** on a quantity that feeds the FROZEN gate's null. At
        L=330 of 8759 returns a path holds ~26 independent blocks, and that is a different
        bootstrap in character, not a tuning tweak. Not a change to make unilaterally.
      - L 204 → 281 is a **38% longer** block on 26 anchored surfaces. Whether crowns move is
        exactly what the 75–80 min regeneration measures, and AC4 makes an adverse move a
        REGRESSION-class escalation. Spending that compute before the convention question is
        settled would be spending it on bodies that may not ship.

      **OPERATOR RULED 2026-10-10: adopt the Literature convention and KEEP the clamp.** L=281
      for the 26 saturated surfaces; the direction is thesis-safe; the body must state
      unmistakably that L there is the **cap, not the estimate**; then the regeneration and AC4's
      verdict-level direction check. The clamp itself is explicitly not reopened in this story.

      **AC2 DECIDED by the pre-registered criterion, which turned out to select on its own.** On
      the 8 observable surfaces, measured against the current `mean_abs` of 115 (2023 4h):

      | candidate | L | direction vs shipped | literature-faithful? |
      |---|---|---|---|
      | `mean_abs` (as-is) | 115 | — | no: estimates `b` for a mixture no symbol resamples |
      | `standardized_mean_abs` | 89 | **shorter — adverse** | no: still a mixture |
      | `per_symbol_median` | 67 | **shorter — adverse** | yes, but the reduction discards the binding symbol |
      | **`per_symbol_max`** | **124** | **longer — safe** | **yes** |

      `per_symbol_max` is the only candidate that is both most-faithful *and* non-adverse, so the
      criterion picks it without a judgement call — which is what pre-registering it was for. It
      is also the conservative reduction: one L is structural (the stationary-bootstrap index is
      drawn once and shared, ADR-0051 § D1), and a block shorter than any symbol's dependence
      destroys that symbol's structure. On the 26 clamped surfaces it is 281 like everything else,
      so it changes nothing there — the whole effect is on the 8.

      Had the criterion not been written down before the numbers existed, `per_symbol_median`
      (shorter L ⇒ more crowns) would have been available to pick post-hoc. It was not picked
      because it was ruled out in advance, not because it looked wrong afterwards.
- [ ] *(superseded)* Dev: window/m̂/ε/volume fixes + re-baselined pins (AC1-AC3).
- [x] **AC1 + AC2 LANDED 2026-10-10.** `politis_white_block_length` now delegates to
      `PwsdVariant::Literature`, and the `Auto` arm estimates per symbol and reduces by `max`.
      The generator reports `selected_block_length = 281` for `v1-momentum-theta-surface-2023`,
      equal to `literature.per_symbol_max` — self-checked by the diagnostic, which **fired the
      moment the policy changed**, which is what a self-check is for.
      - **AC1's re-baseline requirement turned out to be unnecessary, and that is a correction to
        the AC rather than a shortcut.** It says *"the FP-C1.6 pin is re-baselined to the new
        l_ar1 with the change stated in the assert message"* — but the measurement from the
        previous iteration showed the two conventions **agree** on that fixture (AR(1) L=7,
        i.i.d. L=1 both ways), so the pin does not move and there is nothing to re-baseline. It
        is green unchanged. The AC assumed the fixture would move because the algebra says the
        window widens; on that fixture the m̂ shift cancels it.
      - `PwsdVariant::Shipped` stays reachable on purpose: it is what the superseded
        `mc-robustness-2026-06` bodies were produced with, so the old numbers are superseded
        rather than lost, and `literature_variant_direction_on_the_fp_c1_6_fixtures` still pins
        the old arm at (7, 1).
- [~] **AC4 — regeneration RUNNING 2026-10-10.** All 35 surfaces into the scratchpad, never
      `evidence/`, so the diff census is taken **before** anything lands — the order the
      1-26/1-27/1-28 passes established, and the only order in which "what changed" is a
      measurement rather than a reading. First surface:
      `v1-basis-reversal-fee00bps-theta-surface-2023` → `c8c2d2d1…` at **L=281**, confirming the
      new policy is live in the regeneration and that bodies move.

      **ADR-0045 § D6 is NOT the protocol I assumed, and reading it found an obligation.** AC4
      cites it for the namespace bump; what § D6 actually governs is the reconciliation of the
      **two** anchor systems — FILE-anchors (`anchors.toml`, which cannot see engine drift) and
      IN-TEST re-run anchors (hardcoded `const ANCHOR` strings, which can). § D6.1 makes
      `anchors.toml` canonical and the in-test constants a **derived projection** that is
      re-locked *to* it, never hand-authored against a fresh run.

      So the re-lock is incomplete until every in-test literal projecting from the 35 rows moves
      with them. Derived rather than eyeballed, via `check_determinism_anchors.py`'s scanned set:

      | literal | resolves to | affected? |
      |---|---|---|
      | `theta_surface_reproduction.rs:362` `937cb52f7186…` | `v1-momentum-2023-block-bootstrap-real-fy-mc` (`mc-robustness-2026-06`) | **YES — one of the 35** |
      | `reproducibility_sample_figure.rs:49` `076929bb63d9…` | `btc-yahoo-2024-1d-sma-cross` (`lab-yahoo-realdata-v0.1.1`) | no — different family, no block bootstrap |

      **Exactly one** in-test literal must move with this re-lock. The other 34 surfaces need no
      constant update because `theta_surface_reproduction.rs` reads its expectation from
      `anchors.toml` at run time (`anchored_sha`) — it is already a derived projection by
      construction, which is § D6.1 satisfied structurally rather than by discipline.
- [ ] Review: verify old rows byte-intact, new rows complete, verdict-delta table honest (AC5).

## Dev Notes

- Origin: 1-13 review findings (Blind Hunter H1 + M3, Edge L3, plus the volume-proxy M7 routed here for anchor safety) — full evidence in `1-13-monte-carlo-bootstrap-path-generator.md` § Review Findings.
- Do-not-build register: not implicated (this is gate-credibility maintenance, not alpha-chasing; the E-1 report-only boundary is untouched — L feeds the EXISTING gate's null, no crown-eligibility change).
- The 1-13 pass pinned current behavior (`l_ar1 == 7` assert) precisely so this story's change is loud, not silent.
- Direction-of-change guard (AC4) is the story's honesty core: the re-lock must PROVE the thesis-safe direction claim, not assume it.

### References

- Trace: `REQ-PWSD-FIDELITY-RELOCK-001` (state=`scoped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 1 (Strategy & Backtest Engine (v0-v5 ladder + robustness program))
