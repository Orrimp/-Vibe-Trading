---
slug: dsr-report-only-decision
status: draft
owner: architect
updated: 2026-07-09
---

# The DSR scorecard is report-only — decision + empirical basis (R3-3b)

> **What this doc is.** The v3 close-out phase R3-3b: **surface the D3 DSR decision as
> an explicit, evidenced decision** — the operator has kept the overfitting scorecard
> (Deflated-Sharpe / N_eff / MinBTL) **report-only**, and the crown-eligibility **veto
> stays a ready-but-unbuilt one-line switch**. This is a *decision record*, NOT a
> build and NOT a gate change. It consolidates what is otherwise scattered across
> `spec/v2/v2-architecture.md` §6.0 (D2/D3), ADR-0075, and the P2-2 CI so the next
> session doesn't re-open a settled call.

> **What this doc is NOT.** It does **not** wire a veto, does **not** change the
> FROZEN gate (`crates/backtest/src/bakeoff/{robustness,rank}.rs` stay byte-frozen),
> and does **not** touch anchors. The operator is in lock-it-down mode.

## The decision (D3, kept 2026-07-09)

**The DSR/N_eff/MinBTL scorecard is REPORT-ONLY.** The cockpit shows the crown AND
its deflated-confidence side by side; the operator reads the haircut and decides. No
DSR/PBO disqualifier gates crown eligibility. The **`Scorecard.crown_clears_dsr`
flag stays informational** — a `true iff deflated_sharpe >= 0.95` readout, never a
veto.

This was the recommended durable default at v2 scoping (`v2-architecture.md` §6.0
D2/D3), ratified there, and **re-confirmed by the operator on 2026-07-01 informed by
the P2-2 empirical data** (below), then **kept again in the v3 close-out** (2026-07-09,
`spec/dev-notes/post-v2-scoping-2026-07-09.md` §3 R3-3b): the operator kept report-only
and separately parked CI — a lock-it-down posture, not a change-the-gate posture.

## Why report-only is the honest default (the rationale)

- **Zero FROZEN-gate risk.** A DSR/PBO crown-eligibility veto is a change to the
  gate's *effective crowning behaviour* — it is NOT additive-by-default
  (`v2-architecture.md` §6 D3, CX-3). The frozen bands + `rank_candidates` +
  the ADR-0066 benchmark exemption are byte-frozen and anchor-load-bearing. A veto
  therefore needs its own ADR + explicit operator sign-off; it is not smuggled in as
  "additive."
- **Auditable, no-magic.** Report-only means the operator sees the exact number and
  the exact crown, and makes the call. There is no hidden threshold silently
  rejecting an arm.
- **The threshold derivation is deferred, not fudged.** A veto would need a *stated*
  bar — either the hard `DSR ≥ 0.95` or an ORATIO-derived bar from the operator's
  cost-asymmetry ("a false 'beats-hold' is N× costlier than a miss," CX-4 / D2). That
  derivation only matters *if* a veto is chosen; while report-only, the hard-coded
  0.95 is a display flag, not a gate.

## The empirical basis (P2-2 made the decision non-hypothetical)

The no-alpha-gate null-falsification CI —
`crates/backtest/tests/null_data_no_crown.rs` (feature `advisor-no-alpha-gate-ci`,
`spec/v2/phase-2d/`) — reproduces `run_bakeoff`'s exact per-arm sequence over
deterministic pure-noise processes (GBM / GARCH(1,1) / OU) and empirically established
the **two-layer** property that makes the report-only scorecard load-bearing:

1. **The PRIMARY FROZEN gate alone crowns pure noise ~1 in 5 seeds.** On true-null
   series an active arm occasionally clears the per-candidate FRAGILE filter by
   chance (a documented per-candidate multiple-testing gap — the `is_eligible()`
   property; exactly the reason DSR/N_eff/MinBTL exist). Independently re-observed by
   the tester on a fresh seed draw: GBM 1/5 ActiveWins (`v0.5.rsi`), GARCH 1/5
   ActiveWins (`v0.5.rsi`).
2. **The DSR scorecard caught EVERY chance-crown.** Each observed ActiveWins on noise
   had `crown_clears_dsr = false` (deflated-Sharpe ≈ 0.40–0.78, all < 0.95) — the
   zero-tolerance falsification condition held. The scorecard is the second layer
   that flags what the primary gate misses.

This is precisely why the scorecard is worth shipping **and** why it does not need to
be a veto to be valuable: it is a visible, honest haircut that would have flagged
every noise-crown, presented next to the crown for the operator to weigh. (Caveat:
the P2-2 numbers are on short synthetic series — 150 bootstrap paths vs production
1000, a subset of arms — so production likely rejects *more*, not fewer.)

## The veto is a ready-but-unbuilt one-line switch

The design deliberately left the veto a one-line change for a *future* operator
decision + its own ADR:

- **`crates/backtest/src/bakeoff/scorecard.rs`** — `Scorecard.crown_clears_dsr: bool`
  (`= deflated_sharpe >= DSR_THRESHOLD`, `DSR_THRESHOLD = 0.95`). The doc comment on
  the field/module states it is **"report-only in v2"** and **"Informational, never a
  veto."** `rank.rs` does not read it.
- **To wire a veto (NOT done, NOT authorized here):** `rank_candidates`
  (`crates/backtest/src/bakeoff/rank.rs`) would consult `crown_clears_dsr` (or a
  PBO/ORATIO-derived bar) in its eligibility partition. That is a change to the
  FROZEN gate's effective behaviour ⇒ it requires:
  1. explicit operator sign-off (a values call — the cost-asymmetry N, or "hard
     0.95"),
  2. its own ADR (per `v2-architecture.md` §6 D3, CX-3),
  3. an anchor-impact assessment (the advisor bake-off path runs
     `write_report=false`, so it may be anchor-safe by construction — but a veto that
     changes a crown could change any anchored artifact that routes through the same
     ranking; that must be checked, not assumed), and
  4. a day-1 test proving the veto actually bites (the CLAUDE.md no-op-overlay
     precedent: a switch that is computed but never consulted is the exact failure
     class the e2e-divergence non-negotiable exists to catch).

Until all four land, the switch stays informational.

## Status of the related deferred items (for completeness)

- **PBO via CSCV** — `Scorecard.pbo` is `Option<f64>` and always `None`
  (`scorecard.rs`), deferred to the homogeneous Tune/sweep surface (R2) where CSCV is
  statistically honest. NOT part of R3-3b; NOT wired into the field gate.
- **ORATIO-derived DSR threshold** — moot while report-only; only relevant if a veto
  is chosen (then the bar is derived from the operator's cost-asymmetry statement).

## Decision log

- 2026-06-28 (v2 scoping, operator): D2/D3 ratified — scorecard report-only,
  closed-form N_eff frozen at 24 configs, PBO deferred, the `crown_clears_dsr` veto
  switch left ready. (`spec/v2/v2-architecture.md` §6.0.)
- 2026-07-01 (operator): D3 re-confirmed, informed by the P2-2 empirical CI — kept
  report-only; the cockpit shows crown + low-DSR side by side.
- 2026-07-09 (operator, v3 close-out): kept report-only + parked CI (lock-it-down,
  not change-the-gate). R3-3b = document this decision + its evidence (this doc). The
  veto stays unbuilt.

## References

- `spec/v2/v2-architecture.md` §6.0 (D2/D3), §6 (CX-3/CX-4).
- `spec/architecture/adr/0075-overfitting-scorecard.md` (the report-only scorecard).
- `crates/backtest/src/bakeoff/scorecard.rs` (`crown_clears_dsr`, `DSR_THRESHOLD`).
- `crates/backtest/tests/null_data_no_crown.rs` + `spec/v2/phase-2d/` (the P2-2
  empirical two-layer proof).
- `spec/dev-notes/post-v2-scoping-2026-07-09.md` §3 R3-3, §4 (off-track register row
  on the veto).
