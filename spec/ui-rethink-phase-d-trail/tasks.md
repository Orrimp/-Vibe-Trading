---
slug: ui-rethink-phase-d-trail
status: draft
owner: analyst
updated: 2026-05-20
---

# Tasks — UI rethink Phase D (Trail view)

> Analyst-pass deliverable. M0 acceptance gates are anchored to the
> R/Q/K/H registers in `feature.md`. M-T1 is architect-decomposed
> next pass.

## M0 — Analyst synthesis

- [x] T-A1 — Read dev-note §6 Phase D (scope source-of-truth) + §J4
  (Trail contract) + §1 (current audit-screen audit).
  _Acceptance: feature.md "Why" + "Requirements" anchored to dev-
  note line numbers; no silent scope drift._
- [x] T-A2 — **Q1 schema gap analysis** — read
  `crates/audit/src/journal.rs` +
  `crates/audit/migrations/{001..010}.sql` and confirm which per-
  stage correlation IDs (forecast_id, signal_id, fill_id) are
  already carried. Cite file:line for each finding.
  _Acceptance: feature.md "Q1 — Schema gap closure" section carries
  per-table findings with file:line citations; migration shape
  (mig 011) proposed with column-level detail (R1.1-R1.5)._
  _Verdict: schema gap confirmed and substantial; mig 011 is the
  additive shape, anchor-safe by construction._
- [x] T-A3 — Read predecessor `audit-tick-consumer-envelope v0.1.0`
  shipped 2026-05-20. Confirm `AuditTickStream` / `AuditEvent`
  surface Phase D consumer can rely on.
  _Acceptance: R6.1-R6.3 anchored to `audit_tick_consumer.rs:30-32`
  + `tick.rs:78-81,172-191` shape; no producer-side change required._
- [x] T-A4 — Read Phase C's `screens::live::view`
  (`crates/ui/src/screens/live.rs`) + the existing `audit::view`
  (`crates/ui/src/screens/audit.rs`) row list. Identify the chevron
  insertion points.
  _Acceptance: R5.1 cites `agent_feed.rs:49-97` (Live row list) and
  R5.2 cites the per-row Button in `screens/audit.rs` `table_body`._
- [x] T-A5 — Trace `TcnForecaster` runtime construction path
  (K5 deep-dive).
  _Acceptance: K5 risk in feature.md cites
  `forecast/src/tcn.rs:573` (with_ledger), `:822-829,940-945`
  (emit sites), `strategy/src/tcn_overlay_momentum.rs:170-184`
  (TcnSyncForecaster::load_bs1/bs2), `agent/src/runtime.rs:129-141`
  (build_registry SMA-only path) — confirming the gap that the
  architect spike must close._
- [x] T-A6 — Surface Q1-Q5 with recommended defaults for operator-
  decide.
  _Acceptance: Each Q in feature.md carries a "Default → (X)" line
  with the analyst's justification; operator may accept by taking no
  action (architect proceeds with defaults)._
- [x] T-A7 — Author K1-K7 risk register; H1-H5 hypothesis register;
  non-regression contract; acceptance criteria per milestone.
  _Acceptance: feature.md "K-risk register", "H-hypothesis
  register", "Non-regression contract", "Acceptance criteria"
  sections complete._
- [x] T-A8 — Update trace.toml status. Row `REQ-UI-RETHINK-PHASE-D-001`
  already opened in `proposed` by orchestrator. Analyst pass leaves
  it at `proposed` — architect moves to `accepted` on M-T1
  completion. (Owner column convention; no edit required from this
  pass.)

## M-OD — Operator-decide (between M0 and M-T1)

> Operator may accept all defaults by taking no action; architect
> spawns automatically. Override by responding to the orchestrator
> with the chosen letter per Q.

- [ ] T-OD1 — Q1 schema gap resolution: mig 011 additive shape per
  analyst recommendation. Default: ship.
- [ ] T-OD2 — Q2 trail node ordering. Default: (a) upstream-at-top.
- [ ] T-OD3 — Q3 side-drawer trigger. Default: (a) chevron-click.
- [ ] T-OD4 — Q4 first downstream consumer. Default: (a) trail-
  mirror itself (closes T-D-14 in this brief; bigger K5 budget).
- [ ] T-OD5 — Q5 chevron visibility. Default: (a) every row + lazy
  backfill (H3 idle-CPU gate guards regression).

## M-T1 — Architect decomposition (next pass)

- [ ] T-T1-1 — **K5 spike** (2-day budget): produce ADR amendment to
  `0031-audit-tick-consumer-envelope.md` documenting the chosen
  `TcnForecaster` runtime-wiring shape. Cover the
  `TcnSyncForecaster::with_ledger` builder addition + new
  `build_registry_with_ledger(cfg, ledger)` sibling in
  `agent::runtime`. Cover the determinism gate (backtests must NOT
  arm the tick_bus → `tick.rs:104-107` static-branch tee stays
  dormant).
  _Acceptance: ADR amendment merged before any other M-T1 task
  starts; spike output is a working code path or a fallback decision
  (defer R6.4 to follow-up brief)._
- [ ] T-T1-2 — Mig 011 SQL shape locked: exact columns + indexes per
  R1.1-R1.5. New writer
  `audit::journal::post_forecast_event(...)` signature locked.
  Extended signatures for `post_fill` (adds `fill_id`, `signal_id`)
  and `post_strategy_signal` (adds `forecast_correlation_id`)
  locked.
- [ ] T-T1-3 — Architect decomposes R1-R7 into ordered T-tasks with
  acceptance gates per task.
- [ ] T-T1-4 — Architect confirms trail-mirror location: `crates/ui`
  vs. `crates/reflection` (R6.1).
- [ ] T-T1-5 — Spec-lint clean.

## M-FINAL — Tester sweep

- [ ] T-F1 — `cargo fmt --check` + `cargo clippy --workspace -- -D
  warnings` exit 0.
- [ ] T-F2 — `cargo test --workspace --lib` 100% PASS.
- [ ] T-F3 — New snapshot baselines for:
  - `trail__steady_state` (trail screen list mode — byte-identical
    to legacy `audit::view`; R7.1 gate)
  - `trail__side_drawer_open` (trail mode + drawer payload)
  - `live__recent_activity_with_chevron` (Live screen with the new
    chevron — R5.1)
- [ ] T-F4 — `scripts/verify_anchors.sh` → 22/22 PASS —
  non-negotiable. (H2 falsification gate.)
- [ ] T-F5 — `cockpit-smoke` → 0 panic lines (R7.3).
- [ ] T-F6 — Cockpit-performance v1.0.0 idle-CPU floor ≤13.1%
  preserved under the new broadcast subscriber + universal chevron
  (R7.4, H3 falsification gate). Budget: ≤13.6% (0.5% headroom).
- [ ] T-F7 — Counter
  `reflection_audit_tick_seen_total{variant="ForecastEmitted"}`
  observed ≥1 in a paper-mode TCN-overlay smoke (K7 gate). Confirms
  R6.4/R6.5 wiring is live.
- [ ] T-F8 — Trail-mirror compound-dispatch round-trip test (R5,
  K6). Pattern: `Message::OpenTrailFor(uuid)` → assert
  `current_screen == Trail && trail_screen_state.selected_audit_id ==
  Some(uuid)`. Mirrors `state.rs:2489-2498` precedent.
- [ ] T-F9 — H5 backfill-latency benchmark: SQLite p99 first-open
  trail reconstruction < 50 ms at ≥10⁵ audit rows.
- [ ] T-F10 — Author
  `spec/ui-rethink-phase-d-trail/reports/test-final-<YYYY-MM-DD>.md`
  per the rust-test template.

## Notes

- Predecessor: `ui-rethink-phase-c-sidebar-ia v0.1.0`. Phase A/B/C
  surfaces stay byte-identical (R7.2).
- Estimated cost (per dev-note §6 Phase D): ~3-4 weeks. K5 spike
  (2 days) dominates the early risk; mig 011 is mechanical
  (mirrors mig 008/009/010 precedent).
- **Closes deferred T-D-14 from audit-tick-consumer-envelope** —
  `TcnForecaster::with_ledger` runtime wiring becomes load-bearing
  in R6.4 / R6.5. Fallback (architect spike fails): defer R6.4 to
  follow-up brief, ship Phase D with R6.1-R6.3 only (mirror reads
  Fill/Signal/KillSwitch ticks; TCN forecasts come from SQL
  backfill only).
- Q1 schema gap finding is **the** load-bearing analyst output of
  this pass — mig 011 is required, anchor-safe by construction,
  and the writers it enables are precisely what the trail-mirror
  joins on.
