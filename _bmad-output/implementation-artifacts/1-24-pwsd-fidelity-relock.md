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
- [ ] Dev: window/m̂/ε/volume fixes + re-baselined pins (AC1-AC3).
- [ ] Re-run + re-lock: regenerate, anchor under the new namespace, errata note (AC4).
- [ ] Review: verify old rows byte-intact, new rows complete, verdict-delta table honest (AC5).

## Dev Notes

- Origin: 1-13 review findings (Blind Hunter H1 + M3, Edge L3, plus the volume-proxy M7 routed here for anchor safety) — full evidence in `1-13-monte-carlo-bootstrap-path-generator.md` § Review Findings.
- Do-not-build register: not implicated (this is gate-credibility maintenance, not alpha-chasing; the E-1 report-only boundary is untouched — L feeds the EXISTING gate's null, no crown-eligibility change).
- The 1-13 pass pinned current behavior (`l_ar1 == 7` assert) precisely so this story's change is loud, not silent.
- Direction-of-change guard (AC4) is the story's honesty core: the re-lock must PROVE the thesis-safe direction claim, not assume it.

### References

- Trace: `REQ-PWSD-FIDELITY-RELOCK-001` (state=`scoped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 1 (Strategy & Backtest Engine (v0-v5 ladder + robustness program))
