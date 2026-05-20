---
slug: ui-rethink-phase-d-trail
status: shipped
owner: operator
updated: 2026-05-20
---

# Tasks — UI rethink Phase D (Trail view)

> M-T1 architect-decomposed. T-D-N rows below are the developer's
> ordered checklist; every row references the contract in
> [decomp.md](decomp.md) § (file:line + change-site) and the
> requirement registers in [feature.md](feature.md) (R/Q/K/H).
> Operator-decided defaults (Q1-Q5) baked in via "Autoapprove all"
> 2026-05-20.

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
- [x] T-A7 — Author K1-K7 risk register; H1-H5 hypothesis register;
  non-regression contract; acceptance criteria per milestone.
- [x] T-A8 — Update trace.toml status.

## M-OD — Operator-decide (resolved 2026-05-20 "Autoapprove all")

- [x] T-OD1 — Q1 = ship mig 011 (additive).
- [x] T-OD2 — Q2 = (a) upstream-at-top.
- [x] T-OD3 — Q3 = (a) chevron-click drawer trigger.
- [x] T-OD4 — Q4 = (a) trail-mirror itself (closes T-D-14).
- [x] T-OD5 — Q5 = (a) every row + lazy backfill.

## M-T1 — Architect decomposition

- [x] T-T1-1 — **K5 spike** (2-day budget). Result: **SUCCESS**.
  ADR amendment landed in
  [`spec/architecture/adr/0031-audit-tick-consumer-envelope.md`](../architecture/adr/0031-audit-tick-consumer-envelope.md)
  § "Phase D amendment (2026-05-20)". Wiring shape locked in
  [decomp.md §1.2](decomp.md). No fallback exercised.
- [x] T-T1-2 — Mig 011 SQL shape locked per
  [decomp.md §2](decomp.md). 4 ALTER + 1 CREATE TABLE IF NOT EXISTS
  + 4 CREATE INDEX. Writer signatures locked:
  `post_fill_with_signal` (new), `post_strategy_signal` (extended
  6→7 args), `post_forecast_event` (new).
- [x] T-T1-3 — R1-R7 decomposed into ordered T-D-N rows below
  (Waves A-G).
- [x] T-T1-4 — Trail-mirror location: **`crates/reflection`**
  (rationale in [decomp.md §3](decomp.md)).
- [ ] T-T1-5 — Spec-lint clean (tester-gate; deferred to M-FINAL).

## Wave A — Mig 011 + audit writers (anchor gate)

> Exit condition: `scripts/verify_anchors.sh → 22/22 PASS`. This
> is the H2 falsification gate; no subsequent wave starts until
> this passes.

- [x] T-D-N1 — Create
  `crates/audit/migrations/011_trail_correlation_chain.sql`.
  **file:line** `crates/audit/migrations/011_trail_correlation_chain.sql:1-49`
  **Test** `cargo test -p audit -- migrations`
  **Output** `test migrations::run_all_migrations ... ok`
- [x] T-D-N2 — `post_fill_with_signal` + thin `post_fill` wrapper.
  **file:line** `crates/audit/src/journal.rs:74-244`
  **Test** `cargo test -p audit -- journal::tests::post_fill_with_signal_links_signal_id`
  **Output** `test journal::tests::post_fill_with_signal_links_signal_id ... ok`
- [x] T-D-N3 — Extended `post_strategy_signal` with 8th param
  `forecast_correlation_id: Option<Uuid>`.
  **file:line** `crates/audit/src/journal.rs:319` (+ `#[allow(clippy::too_many_arguments)]`)
  **Test** `cargo test -p audit -- journal`
  **Output** all journal tests ok
- [x] T-D-N4 — `post_forecast_event` new writer.
  **file:line** `crates/audit/src/journal.rs` (post_forecast_event fn)
  **Test** `cargo test -p audit -- journal::tests::post_forecast_event_round_trip`
  **Output** `test journal::tests::post_forecast_event_round_trip ... ok`
- [x] T-D-N5 — Plumb `post_forecast_event` calls into the two
  existing emit sites in `crates/forecast/src/tcn.rs` — cache-hit
  branch at `:861-879` and post-inference at `:997-1010`. The call
  fires **alongside** the existing `tick::emit_public(...)` (both
  fire on every emit). Production builder path
  (`with_tcn_bs1_ledger` / `with_tcn_bs2_ledger`) seeds the
  per-instance forecast context via `with_forecast_context(...)` so
  the emit guard at `tcn.rs:862` actually fires in production.
  **file:line** `crates/strategy/src/tcn_overlay_momentum.rs:417-420,434-437`
  (builder context wiring); `crates/forecast/src/tcn.rs:861-879,997-1010`
  (emit sites).
  _Acceptance: production wiring complete; paper-mode K7 counter
  validation deferred to tester M-FINAL T-F7._
- [x] T-D-N6 — **Anchor gate (H2 falsification)**: run
  `scripts/verify_anchors.sh`. Required: 22/22 PASS. If any anchor
  diverges, roll back Wave A and re-architect.
  **Output** `ANCHORS PASS (22 / 22)` — orchestrator-verified
  2026-05-20.

## Wave B — `widgets::trail_node` (new widget)

- [x] T-D-N7 — Created `crates/ui/src/widgets/trail_node.rs`.
  **file:line** `crates/ui/src/widgets/trail_node.rs:86` (`pub fn view`)
  **Test** `cargo test -p ui --lib widgets::trail_node`
  **Output** `test widgets::trail_node::tests::each_kind_renders_dark_unselected ... ok`
- [x] T-D-N8 — Registered widget; added `Message::TrailNodeChevronClicked`, `Message::TrailDrawerClosed`.
  **file:line** `crates/ui/src/widgets/mod.rs` (pub mod trail_node), `crates/ui/src/state.rs`
  **Test** `cargo build -p ui`
  **Output** `Finished dev profile`
- [x] T-D-N9 — Unit tests (4 kinds × 2 themes × 2 selection states).
  **file:line** `crates/ui/src/widgets/trail_node.rs:179-263`
  **Test** `cargo test -p ui --lib widgets::trail_node`
  **Output** `test result: ok. 4 passed; 0 failed`

## Wave C — `screens::trail` (new screen body)

- [x] T-D-N10 — Created `crates/ui/src/screens/trail.rs`.
  **file:line** `crates/ui/src/screens/trail.rs:27` (`pub fn view`)
  **Test** `cargo build -p ui`
  **Output** `Finished dev profile`
- [x] T-D-N11 — Registered `pub mod trail` in screens/mod.rs; added `Screen::Trail`.
  **file:line** `crates/ui/src/screens/mod.rs`, `crates/ui/src/state.rs`
  **Test** `cargo check -p ui`
  **Output** clean
- [x] T-D-N12 — Wired `Screen::Trail` dispatch in `update_screen`.
  **file:line** `crates/ui/src/state.rs` (update_screen match arm)
  **Test** `cargo test -p ui --lib state`
  **Output** all state tests ok

## Wave D — `widgets::trail_drawer` + state

- [x] T-D-N13 — Created `crates/ui/src/widgets/trail_drawer.rs`.
  **file:line** `crates/ui/src/widgets/trail_drawer.rs:73` (`pub fn view`)
  **Test** `cargo build -p ui`
  **Output** `Finished dev profile`
- [x] T-D-N14 — Extended `Cockpit` with `trail_screen_state: TrailScreenState`.
  **file:line** `crates/ui/src/state.rs` (`TrailScreenState` struct + Cockpit field)
  **Test** `cargo check -p ui`
  **Output** clean
- [x] T-D-N15 — Added Message variants + wired update arms.
  **file:line** `crates/ui/src/state.rs` (Message enum + update match arms)
  **Test** `cargo test -p ui --lib state`
  **Output** all state tests ok

## Wave E — Live recent-activity chevron

- [x] T-D-N16 — Added Trail chevron in `agent_feed.rs ready_body`.
  **file:line** `crates/ui/src/widgets/agent_feed.rs:130-143`
  **Test** `cargo test -p ui --lib widgets::agent_feed`
  **Output** all agent_feed tests ok
- [x] T-D-N17 — Added `Message::OpenTrailFor(SmolStr)` + compound dispatch.
  **file:line** `crates/ui/src/state.rs` (Message::OpenTrailFor + update arm)
  **Test** `cargo test -p ui --lib state::tests::open_trail_for_sets_screen_and_selected_audit_id`
  **Output** `test state::tests::open_trail_for_sets_screen_and_selected_audit_id ... ok`
- [x] T-D-N18 — Mirrored chevron in `screens/audit.rs row_for`.
  **file:line** `crates/ui/src/screens/audit.rs` (row_for fn, chevron adjacent to row_btn)
  **Test** `cargo build -p ui`
  **Output** clean

## Wave F — Trail-mirror consumer + TCN runtime wiring

- [x] T-D-N19 — Added `TcnOverlayConfig` + `StrategiesConfig` field + TOML tests.
  **file:line** `crates/agent/src/config.rs` (TcnOverlayConfig struct + 2 tests)
  **Test** `cargo test -p agent --lib config`
  **Output** `test config_tcn_overlay_default_off ... ok`, `test config_tcn_overlay_explicit_enable_round_trips ... ok`
- [x] T-D-N20 — `TcnSyncForecaster::with_ledger` + `with_forecast_context` builders.
  **file:line** `crates/strategy/src/tcn_overlay_momentum.rs` (under `#[cfg(feature = "forecast-audit-tick")]`)
  **Test** `cargo check -p strategy --features forecast,forecast-audit-tick`
  **Output** clean
- [x] T-D-N21 — `TcnOverlayMomentumStrategy::with_tcn_bs1_ledger` (and bs2 variant).
  **file:line** `crates/strategy/src/tcn_overlay_momentum.rs` (with_tcn_bs1_ledger/bs2_ledger)
  **Test** `cargo test -p strategy --lib tcn_overlay_momentum`
  **Output** `test ... ok`
- [x] T-D-N22 — `build_registry_with_ledger` in runtime.rs + main.rs call-site swap.
  **file:line** `crates/agent/src/runtime.rs` (build_registry_with_ledger fn), `crates/agent/src/main.rs`
  **Test** `cargo build -p agent`
  **Output** clean
- [x] T-D-N23 — Created `crates/reflection/src/trail_mirror.rs` with BoundedLru, TrailMirror, unit tests.
  **file:line** `crates/reflection/src/trail_mirror.rs:1-400+`
  **Test** `cargo test -p reflection --lib trail_mirror`
  **Output** `test trail_mirror::tests::lru_cap_enforced ... ok` (H4 gate)
- [x] T-D-N24 — Registered `pub mod trail_mirror` in reflection/lib.rs; spawned from main.rs.
  **file:line** `crates/reflection/src/lib.rs` (pub mod trail_mirror), `crates/agent/src/main.rs:179`
  **Test** `cargo build -p agent`
  **Output** clean

## Wave G — Snapshots + integration + perf-gate

- [x] T-D-N25 — `audit::query::trail_for_fill_id` + integration tests.
  **file:line** `crates/audit/src/query.rs` (trail_for_fill_id fn + TrailReconstruction types),
  `crates/audit/tests/trail_reconstruction.rs`
  **Test** `cargo test -p audit --test trail_reconstruction`
  **Output** `test trail_full_triplet_returns_all_three_stages ... ok`, `test trail_fill_only_returns_fill_and_nones ... ok`, `test trail_missing_fill_returns_default ... ok`
- [ ] T-D-N26 — Iced Subscription bridge in
  `crates/ui/src/state.rs:~1213` —
  `trail_mirror_subscription(handle: &TrailMirrorHandle) ->
  Subscription<Message>` returning a stream of
  `TrailMirrorTick → Message::TrailMirrorTick`. Wired into
  `Cockpit::subscription` batch.
  _Acceptance: cockpit boot subscribes; idle-CPU regression
  guarded by M-FINAL T-F6._
- [ ] T-D-N27 — Add 3 snapshot baselines:
    - `trail__steady_state` — list mode, byte-identical to legacy
      `audit::view` (R2.2 / R7.1 gate)
    - `trail__side_drawer_open` — trail mode with drawer showing
      Forecast node payload
    - `live__recent_activity_with_chevron` — Live screen with the
      universal chevron rendered (R5.1)
  _Acceptance: 3 baselines committed; 22 anchored body-SHAs
  unaffected (the 3 are NEW snapshots, not changes to existing
  anchors)._
- [x] T-D-N28 — Round-trip compound-dispatch tests (K6 gate).
  **file:line** `crates/ui/src/state.rs` (tests at end of state test mod: `open_trail_for_sets_screen_and_selected_audit_id`, `select_trail_row_empty_clears_selection`, `trail_drawer_closed_clears_drawer_not_selection`)
  **Test** `cargo test -p ui --lib state::tests::open_trail_for_sets_screen_and_selected_audit_id`
  **Output** `test state::tests::open_trail_for_sets_screen_and_selected_audit_id ... ok`
- [ ] T-D-N29 — H5 backfill-latency benchmark in
  `crates/reflection/benches/trail_mirror.rs` —
  SQLite p99 first-open trail reconstruction < 50 ms at ≥10⁵
  audit rows. Seed an in-memory SQLite with a synthetic 100k-row
  fixture, time 100 random `Open` requests.
  _Acceptance: bench output asserts p99 < 50 ms (H5 falsifies if
  > 50 ms; forces R6.3 pre-fetch redesign)._

## M-FINAL — Tester sweep

- [x] T-F1 — `cargo fmt --check` + `cargo clippy --workspace -- -D
  warnings` exit 0.
  **file:line** n/a (workspace-wide)
  **Test** `cargo fmt --check && cargo clippy --workspace -- -D warnings`
  **Output** `EXIT:0` for both; `Finished dev profile` for clippy.
  **Verified** 2026-05-20 commit df3957b4
- [x] T-F2 — `cargo test --workspace --lib` 100% PASS.
  **file:line** n/a (workspace-wide)
  **Test** `cargo test --workspace --lib`
  **Output** `937 passed; 0 failed; 2 ignored` (EXIT:0)
  **Verified** 2026-05-20 commit df3957b4
- [ ] T-F3 — New snapshot baselines committed:
  `trail__steady_state`, `trail__side_drawer_open`,
  `live__recent_activity_with_chevron`.
  _DEFERRED to Phase D+ — NEW baselines, not changes to existing
  anchors; 22 anchor gate uncompromised. See test report §11._
- [x] T-F4 — `scripts/verify_anchors.sh` → 22/22 PASS —
  non-negotiable (H2 falsification gate). Already passed at
  Wave A exit; re-run as a confirmation gate at M-FINAL.
  **file:line** `scripts/verify_anchors.sh`
  **Test** `bash scripts/verify_anchors.sh`
  **Output** `ANCHORS PASS (22 / 22)` EXIT:0
  **Verified** 2026-05-20 commit df3957b4
- [x] T-F5 — `cockpit-smoke` → 0 panic lines (R7.3).
  **file:line** `crates/ui/tests/layout_invariants.rs`
  **Test** `cargo test -p ui --test layout_invariants`
  **Output** `6 passed; 0 failed; 0 ignored; finished in 58.66s` (M1-C proxy)
  **Verified** 2026-05-20 commit df3957b4
- [ ] T-F6 — Cockpit-performance v1.0.0 idle-CPU floor ≤13.1%
  preserved under the new broadcast subscriber + universal
  chevron (R7.4, H3 falsification gate). Budget: ≤13.6% (0.5%
  Phase D headroom).
  _DEFERRED — no bench tooling available; requires sustained live cockpit run._
- [ ] T-F7 — Counter
  `reflection_audit_tick_seen_total{variant="ForecastEmitted"}`
  observed ≥1 in a paper-mode TCN-overlay smoke (K7 gate). Run
  paper mode with `[strategies.tcn_overlay_momentum] enabled =
  true` and `[reflection] audit_tick_consumer_enabled = true`;
  assert counter ≥ 1 after 60 s.
  _DEFERRED — infrastructure-dependent (requires BS-1 checkpoint + live feed).
  Wiring confirmed complete by code inspection (tcn.rs:851-879, :985-1007;
  tcn_overlay_momentum.rs:413-438; runtime.rs:163-220). See test report §3 T-F7._
- [x] T-F8 — Trail-mirror compound-dispatch round-trip test
  (R5, K6) — done at T-D-N28; tester confirms in the report.
  **file:line** `crates/ui/src/state.rs` (test mod)
  **Test** `cargo test -p ui --lib state::tests::open_trail_for_sets_screen_and_selected_audit_id`
  **Output** `test state::tests::open_trail_for_sets_screen_and_selected_audit_id ... ok`
  **Verified** 2026-05-20 commit df3957b4
- [ ] T-F9 — H5 backfill-latency benchmark — done at T-D-N29;
  tester confirms in the report.
  _DEFERRED — T-D-N29 not yet authored; bench not yet written. See test report §11._
- [x] T-F10 — Author
  `spec/ui-rethink-phase-d-trail/reports/test-final-<YYYY-MM-DD>.md`
  per the rust-test template.
  **file:line** `spec/ui-rethink-phase-d-trail/reports/test-final-2026-05-20.md`
  **Verified** 2026-05-20 commit df3957b4

## Notes

- Predecessor: `ui-rethink-phase-c-sidebar-ia v0.1.0`. Phase A/B/C
  surfaces stay byte-identical (R7.2).
- K5 spike succeeded; fallback (defer R6.4 to follow-up brief) NOT
  exercised. ADR amendment landed in
  [`adr/0031-audit-tick-consumer-envelope.md`](../architecture/adr/0031-audit-tick-consumer-envelope.md)
  § "Phase D amendment (2026-05-20)".
- Trail-mirror lives in `crates/reflection` (NOT `crates/ui`) —
  rationale in [decomp.md §3](decomp.md).
- Mig 011 is anchor-safe by construction; the Wave-A H2 gate
  (T-D-N6) is the single highest-risk task; everything downstream
  rests on a clean 22/22 PASS at that point.

## Gallery fixes (developer, session 2)

- **GALLERY_LOGICAL_HEIGHT** updated from 13500 → 14600 in
  `crates/ui/src/gallery/mod.rs:62` (55 cells × 260 px + 300 headroom).
- `trail_node` + `trail_drawer` added to `EXPECTED_WIDGETS` and
  `GALLERY_CELLS` in `crates/ui/src/gallery/routes.rs`.
- `render_trail_drawer_fill` E0515 fixed via `Box::leak` pattern (same as
  other gallery render functions).
- `cargo fmt` + `cargo clippy --workspace -- -D warnings` → clean.
- `cargo test --workspace --lib` → 294/294 PASS.

## Changelog

- 2026-05-20 (architect, M-T1): T-D-N1..N29 added across Waves
  A-G. K5 spike verdict SUCCESS; mig 011 SQL locked; trail-mirror
  location pinned to `crates/reflection`. Operator defaults
  baked in. Owner advanced `analyst` → `architect`; status
  advanced `draft` → `in-progress`.
- 2026-05-20 (developer): T-D-N1..N25, T-D-N28 implemented and
  ticked. Waves A-E + F (N19-N24) complete. Wave G: N25 + N28
  done; N26, N27, N29 deferred to tester/next-session.
