---
slug: ui-rethink-phase-d-trail
status: in-progress
owner: architect
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

- [ ] T-D-N1 — Create
  `crates/audit/migrations/011_trail_correlation_chain.sql` with
  the exact SQL from [decomp.md §2.1](decomp.md). 4 ALTERs +
  1 CREATE + 4 INDEXes; header comment block per the mig 010
  precedent.
  _Acceptance: `cargo test -p audit -- migrations` runs the
  migration on a fresh in-memory SQLite; no errors. (R1.1-R1.4)._
- [ ] T-D-N2 — Refactor `crates/audit/src/journal.rs:74-244` —
  introduce `post_fill_with_signal(ledger, fill, venue,
  strategy_id, signal_id)` per [decomp.md §2.2](decomp.md) and
  make the existing `post_fill` a thin
  `post_fill_with_signal(.., None)` wrapper. INSERT grows from 5
  to 7 columns (add `fill_id`, `signal_id`).
  _Acceptance: existing `post_fill` callers (grep — zero outside
  test code) unaffected; new `post_fill_with_signal` writes both
  new columns when `Some`. (R1.1 + R1.2)._
- [ ] T-D-N3 — Extend `post_strategy_signal` at
  `crates/audit/src/journal.rs:293-301` — add 7th param
  `forecast_correlation_id: Option<Uuid>`; bind site at
  `journal.rs:339-355` grows from 10 to 11 columns.
  _Acceptance: All existing test callers (3 sites in
  `journal.rs:2049-2192`) updated to pass `None`; non-test callers
  (zero per grep) unaffected. (R1.3)._
- [ ] T-D-N4 — Add new writer
  `audit::journal::post_forecast_event(ledger, &overlay,
  &strategy_id, &symbol, cache_hit) -> Result<(), LedgerError>`
  per [decomp.md §2.2](decomp.md). `INSERT OR IGNORE` (idempotent
  on `correlation_id` PK).
  _Acceptance: round-trip unit test — write overlay → query
  `SELECT * FROM forecast_events WHERE correlation_id = ?` returns
  the row with the 8 expected columns. (R1.4)._
- [ ] T-D-N5 — Plumb `post_forecast_event` calls into the two
  existing emit sites in `crates/forecast/src/tcn.rs` — cache-hit
  branch at `:822-831` and post-inference at `:937-947`. The call
  fires **alongside** the existing `tick::emit_public(...)` (both
  fire on every emit). `request.strategy_id` and `request.symbol`
  are already in scope at both sites.
  _Acceptance: paper-mode smoke shows `forecast_events` row count
  > 0 after one TCN inference; the existing `ForecastEmitted` tick
  still fires (no regression in
  `reflection_audit_tick_seen_total`). (R1.4)._
- [ ] T-D-N6 — **Anchor gate (H2 falsification)**: run
  `scripts/verify_anchors.sh`. Required: 22/22 PASS. If any anchor
  diverges, roll back Wave A and re-architect.
  _Acceptance: 22/22 PASS. Block Wave B until clean._

## Wave B — `widgets::trail_node` (new widget)

- [ ] T-D-N7 — Create `crates/ui/src/widgets/trail_node.rs`
  exposing `pub fn view(node: &TrailNode, selected: bool,
  mode: ThemeMode) -> Element<'_, Message>` per R3.1-R3.5.
  Pure render; no state. Node types: Fill / Signal / Forecast /
  LLM-debate (placeholder).
  _Acceptance: Each kind renders timestamp (`HH:MM:SS.μμμ` —
  reuse `agent_feed::short_time`), actor label, headline,
  chevron. Selected variant uses
  Lumen `ACCENT_500` border ring or existing `focus_ring`._
- [ ] T-D-N8 — Register widget in `crates/ui/src/widgets/mod.rs`:
  `pub mod trail_node;`. Add `Message::TrailNodeChevronClicked(
  TrailNodeKind)` variant to the Message enum in
  `crates/ui/src/state.rs` (Phase C `OpenStrategyInLab`
  precedent).
  _Acceptance: `cargo check -p ui` clean; widget snapshot-tests
  (added in Wave G) exercise all four node kinds._
- [ ] T-D-N9 — Unit tests for `trail_node::view` — 4 cases (one
  per node kind), 2 themes (light / dark), 2 selection states.
  _Acceptance: `cargo test -p ui widgets::trail_node` 100% PASS._

## Wave C — `screens::trail` (new screen body)

- [ ] T-D-N10 — Create `crates/ui/src/screens/trail.rs` exposing
  `pub fn view(model: &Cockpit, mode: ThemeMode) -> Element<'_>`
  per R2.1-R2.5. **List mode** = verbatim delegation to
  `screens::audit::view(model, mode)` for byte-identical
  rendering (R2.2 gate). **Trail mode** = trail node stack +
  drawer + back-to-list breadcrumb.
  _Acceptance: in list mode (cold-start), snapshot of `Screen::Trail`
  is byte-identical to legacy `Screen::Audit` (R7.1 / R10.1 gate)._
- [ ] T-D-N11 — Register screen module in
  `crates/ui/src/screens/mod.rs`: `pub mod trail;`.
  Add active variant `Screen::Trail` in `state.rs` Screen enum;
  demote existing `Screen::Audit` to deprecated-alias status
  pointing at `Trail` (already partially wired at L84 — confirm
  the Phase A precedent shape).
  _Acceptance: `cargo check -p ui` clean; the Phase C
  3-group sidebar's "Trail" entry routes to `Screen::Trail`._
- [ ] T-D-N12 — Wire `update_screen` dispatch at
  `state.rs:~1549` — `Screen::Trail` → `screens::trail::view`;
  preserve `Screen::Audit` alias → same body.
  _Acceptance: round-trip test `SwitchScreen(Screen::Audit)` ends
  up rendering `screens::trail::view` (alias preservation)._

## Wave D — `widgets::trail_drawer` + state

- [ ] T-D-N13 — Create `crates/ui/src/widgets/trail_drawer.rs`
  exposing `pub fn view(payload, mode) -> Element<'_, Message>`
  per R4.1-R4.4. Renders Fill (JSON pretty-print), Signal (row
  dump), Forecast (row dump + summary), LLM-placeholder
  ("(no transcript recorded)") bodies. Reuses
  `RIGHT_RAIL_WIDTH_PX` from `shell.rs` (no new layout token).
  _Acceptance: snapshot `trail__side_drawer_open` covers all 4
  body variants in both themes._
- [ ] T-D-N14 — Extend `Cockpit` struct in
  `crates/ui/src/state.rs` with `trail_screen_state:
  TrailScreenState`. Fields per [decomp.md §4 Wave D](decomp.md):
  `selected_audit_id: Option<SmolStr>`, `drawer_selected_node:
  Option<TrailNodeKind>`. LRU cache lives in the trail-mirror
  (Wave F), NOT in `TrailScreenState`.
  _Acceptance: `cargo check -p ui` clean; default-constructed
  state has `selected_audit_id == None` (list mode cold-start, R2.5)._
- [ ] T-D-N15 — Add Message variants in `state.rs`:
  `Message::SelectTrailRow(SmolStr)` (internal),
  `Message::TrailDrawerClosed`. Wire update arms — chevron-click
  on a row sets `selected_audit_id`; chevron-click on a node sets
  `drawer_selected_node`; drawer-close clears
  `drawer_selected_node` only (selection survives).
  _Acceptance: round-trip unit tests for each Message arm._

## Wave E — Live recent-activity chevron

- [ ] T-D-N16 — Modify `crates/ui/src/widgets/agent_feed.rs:49-97`
  (`ready_body`) — add per-row Trail chevron Button adjacent to
  the existing transparent row Button at L62-65. Chevron emits
  `Message::OpenTrailFor(audit_id)`. Universal visibility per Q5
  (every row, even pre-mig-011 rows that gracefully degrade via
  R3.4's empty-stage rendering).
  _Acceptance: `live__recent_activity_with_chevron` snapshot
  baseline added; H3 hypothesis (idle-CPU neutral)
  guards the regression at M-FINAL T-F6._
- [ ] T-D-N17 — Add `Message::OpenTrailFor(SmolStr)` to the
  Message enum + update arm in `state.rs`. Expand to compound
  dispatch `SelectTrailRow(id)` + `SwitchScreen(Trail)` per R5.1
  (Phase C `OpenStrategyInLab` precedent at L822 / L2489-2498
  round-trip test).
  _Acceptance: round-trip test `OpenTrailFor(uuid)` →
  `current_screen == Screen::Trail &&
  trail_screen_state.selected_audit_id == Some(uuid)` (K6
  mitigation; M-FINAL T-F8 gate)._
- [ ] T-D-N18 — Mirror change in
  `crates/ui/src/screens/audit.rs:316` (`table_body`) — per-row
  Trail chevron Button sibling of the existing row Button.
  Chevron emits same `Message::OpenTrailFor`. Mutual-exclusivity
  is iced layout-order based (chevron hit-box wins).
  _Acceptance: clicking the chevron on a list-mode row in trail
  screen transitions to trail mode for that row (R5.2)._

## Wave F — Trail-mirror consumer + TCN runtime wiring

- [ ] T-D-N19 — Add config knob in
  `crates/agent/src/config.rs` — new struct `TcnOverlayConfig {
  pub enabled: bool }` with `Default::enabled = false`. Add to
  `StrategiesConfig`. Round-trip TOML parse test mirroring the
  `[signal_log]` precedent at L984-1014.
  _Acceptance: default `agent.toml` parses without the new
  section; explicit `[strategies.tcn_overlay_momentum] enabled =
  true` toggles the flag._
- [ ] T-D-N20 — Implement
  `TcnSyncForecaster::with_ledger(self, ledger: audit::Ledger) ->
  Self` per [decomp.md §1.2](decomp.md) at
  `crates/strategy/src/tcn_overlay_momentum.rs:~184`
  (sibling of `load_bs1` / `load_bs2`). `#[cfg(feature =
  "audit-tick")]`. Forwards to
  `forecast::tcn::TcnForecaster::with_ledger` at
  `forecast/src/tcn.rs:573`.
  _Acceptance: `cargo check -p strategy --features
  forecast,audit-tick` clean._
- [ ] T-D-N21 — Add
  `TcnOverlayMomentumStrategy::with_tcn_bs1_ledger(base,
  ledger)` at `crates/strategy/src/tcn_overlay_momentum.rs:~352`
  mirroring `with_tcn_bs1` at L348-352; threads `ledger` into the
  `TcnSyncForecaster` via the new `with_ledger` builder.
  _Acceptance: `cargo test -p strategy --features
  forecast,audit-tick` 100% PASS; new builder is unit-test-covered._
- [ ] T-D-N22 — Implement
  `agent::runtime::build_registry_with_ledger(cfg, ledger)` at
  `crates/agent/src/runtime.rs:~142` (sibling of
  `build_registry`). Guarded TCN registration per
  [decomp.md §1.2](decomp.md). Swap call site at
  `crates/agent/src/main.rs:184-186` to the new sibling (paper
  mode only).
  _Acceptance: backtests / tests continue to call
  `build_registry(cfg)` (zero-ledger); paper-mode binary builds
  the registry with the tick-bus-armed ledger threaded through._
- [ ] T-D-N23 — Create `crates/reflection/src/trail_mirror.rs`
  per [decomp.md §3](decomp.md). Struct
  `TrailMirror` holds:
    - `stream: AuditTickStream` (subscribed via
      `AuditTickStream::new(rx, "ui_trail_mirror")`)
    - `ledger: Arc<audit::Ledger>` (for SQL backfill)
    - `lru: LruCache<SmolStr, ReconstructedTrail>` (N=16)
    - Request/response channels
      (`mpsc::Receiver<TrailMirrorRequest>` +
      `broadcast::Sender<TrailMirrorTick>`).
  _Acceptance: unit test — subscribe to a stub bus, write 16+
  unique Open requests, assert LRU cap = 16 (H4 gate)._
- [ ] T-D-N24 — Register `trail_mirror` in
  `crates/reflection/src/lib.rs` (`pub mod trail_mirror;`).
  Spawn the mirror task from
  `crates/agent/src/main.rs` adjacent to the existing
  `audit_tick_consumer` spawn site (mirror the same
  `audit_tick_consumer_enabled` cfg gate convention).
  _Acceptance: paper-mode boot logs show
  `target: "trail_mirror" "subscribed"` line in tracing output._

## Wave G — Snapshots + integration + perf-gate

- [ ] T-D-N25 — Add `audit::query::trail_for_fill_id(ledger,
  fill_audit_id)` per [decomp.md §2.3](decomp.md). 4 indexed
  point-lookups against the mig 011 indexes; returns
  `TrailReconstruction { fill, signal, forecast, debate }`.
  _Acceptance: integration test in `crates/audit/tests/` —
  write `Fill+Signal+ForecastEvent` row triplet → query returns
  all three populated, `debate == None`._
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
- [ ] T-D-N28 — Round-trip compound-dispatch test in
  `crates/ui/src/state.rs` test mod per M-FINAL T-F8 (K6 gate).
  Pattern: build `Cockpit`, dispatch `OpenTrailFor(uuid)`, assert
  `current_screen == Screen::Trail &&
  trail_screen_state.selected_audit_id == Some(uuid)`. Mirrors
  `state.rs:2489-2498`.
  _Acceptance: `cargo test -p ui state::tests::open_trail_round_trip`
  PASS._
- [ ] T-D-N29 — H5 backfill-latency benchmark in
  `crates/reflection/benches/trail_mirror.rs` —
  SQLite p99 first-open trail reconstruction < 50 ms at ≥10⁵
  audit rows. Seed an in-memory SQLite with a synthetic 100k-row
  fixture, time 100 random `Open` requests.
  _Acceptance: bench output asserts p99 < 50 ms (H5 falsifies if
  > 50 ms; forces R6.3 pre-fetch redesign)._

## M-FINAL — Tester sweep

- [ ] T-F1 — `cargo fmt --check` + `cargo clippy --workspace -- -D
  warnings` exit 0.
- [ ] T-F2 — `cargo test --workspace --lib` 100% PASS.
- [ ] T-F3 — New snapshot baselines committed:
  `trail__steady_state`, `trail__side_drawer_open`,
  `live__recent_activity_with_chevron`.
- [ ] T-F4 — `scripts/verify_anchors.sh` → 22/22 PASS —
  non-negotiable (H2 falsification gate). Already passed at
  Wave A exit; re-run as a confirmation gate at M-FINAL.
- [ ] T-F5 — `cockpit-smoke` → 0 panic lines (R7.3).
- [ ] T-F6 — Cockpit-performance v1.0.0 idle-CPU floor ≤13.1%
  preserved under the new broadcast subscriber + universal
  chevron (R7.4, H3 falsification gate). Budget: ≤13.6% (0.5%
  Phase D headroom).
- [ ] T-F7 — Counter
  `reflection_audit_tick_seen_total{variant="ForecastEmitted"}`
  observed ≥1 in a paper-mode TCN-overlay smoke (K7 gate). Run
  paper mode with `[strategies.tcn_overlay_momentum] enabled =
  true` and `[reflection] audit_tick_consumer_enabled = true`;
  assert counter ≥ 1 after 60 s.
- [ ] T-F8 — Trail-mirror compound-dispatch round-trip test
  (R5, K6) — done at T-D-N28; tester confirms in the report.
- [ ] T-F9 — H5 backfill-latency benchmark — done at T-D-N29;
  tester confirms in the report.
- [ ] T-F10 — Author
  `spec/ui-rethink-phase-d-trail/reports/test-final-<YYYY-MM-DD>.md`
  per the rust-test template.

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

## Changelog

- 2026-05-20 (architect, M-T1): T-D-N1..N29 added across Waves
  A-G. K5 spike verdict SUCCESS; mig 011 SQL locked; trail-mirror
  location pinned to `crates/reflection`. Operator defaults
  baked in. Owner advanced `analyst` → `architect`; status
  advanced `draft` → `in-progress`.
