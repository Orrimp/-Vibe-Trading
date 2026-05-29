---
slug: architecture-adr-index
status: in-progress
owner: architect
updated: 2026-05-29 (ADR-0050 D1 corrected + D4 added + D3 amended: rt.spawn() is the invariant for HTTP/reqwest paths — rt.enter() guards insufficient for transitive spawn_blocking; see bug-64-arch-revalidation-rt-spawn-2026-05-29.md)
---


# Architecture Decision Records (ADR) — Index

Every non-trivial architectural decision lives here as a dated,
numbered, immutable record. ADRs are *cited* by `spec/trace.toml` rows
in their `arch` field, by feature `## Design` sections, and by code
review checklists.

## Format

See [TEMPLATE.md](TEMPLATE.md). The required frontmatter is:

```yaml
---
adr: NNNN
title: <short imperative title>
status: proposed | accepted | superseded | deprecated
date: <YYYY-MM-DD>
supersedes: <NNNN | none>
superseded-by: <NNNN | none>
---
```

Body sections: `Context`, `Decision`, `Alternatives considered`,
`Consequences`. Keep ADRs short — most should fit in 100-200 lines.
Longer ADRs usually mean the decision is actually two decisions in a
trench coat.

## Numbering rules

- Numbers are assigned by the architect when the ADR is filed and never
  reused. A `superseded` ADR keeps its number; the new ADR gets a new
  one and references the old via `supersedes:`.
- Numbers are zero-padded to 4 digits (`0001`, not `1`).
- Filename pattern: `NNNN-kebab-case-title.md`.

## Registry

(Cross-referenced from `spec/architecture.md` § ADR registry. This is
the canonical table; the parent file links here.)

| ID    | Title                                              | Status     | Date       |
|-------|----------------------------------------------------|------------|------------|
| 0001  | Crate names avoid stdlib collisions                | accepted   | 2026-04-17 |
| 0002  | RNG seeded with ChaCha20 from config seed          | accepted   | 2026-04-17 |
| 0003  | Money math uses Decimal, never f64                 | accepted   | 2026-04-17 |
| 0004  | Audit-DB uses 6-digit fractional-second timestamps | accepted   | 2026-04-18 |
| 0005  | v0 — clean strategy trait shape, no hot-load       | accepted   | 2026-04-17 |
| 0006  | v0.5 — config-driven strategy composition (hot-load A) | accepted | 2026-04-19 |
| 0007  | v1+ — WASM plugin hot-load deferred                | accepted   | 2026-04-19 |
| 0008  | v0.5 — strategy-event journal schema (Q1)          | accepted   | 2026-04-19 |
| 0009  | v0.5 — registry concurrency: parking_lot::RwLock (Q2) | accepted | 2026-04-19 |
| 0010  | v0.5 — ComposedStrategy exit policy: signal-flip only (Q3) | accepted | 2026-04-19 |
| 0011  | v0.5 — cockpit Strategies panel: right column (Q4) | accepted   | 2026-04-19 |
| 0012  | v0.5 — strategy broadcast types in trading_core (Q5) | accepted | 2026-04-19 |
| 0013  | v1 — cross-sectional momentum resolutions (Q1–Q6)  | accepted   | 2026-04-29 |
| 0014  | v1.5a — mean-reversion pairs resolutions (Q1–Q10)  | accepted   | 2026-04-30 |
| 0015  | v1+ — Operator success reports (Q1–Q9)              | accepted   | 2026-05-01 |
| 0016  | v1+ — real-mtm unrealized PnL plumbing (Q1–Q8 + R10) | accepted  | 2026-05-02 |
| 0017  | v1.5b — multi-venue execution scaffolding (Q1–Q12)  | accepted   | 2026-05-03 |
| 0018  | Lumen design adoption — Phase 1 foundation (Q1–Q11) | accepted   | 2026-05-04 |
| 0019  | v2 — LLM strategy foundation (Q4–Q11) — ADR-0019 § Changelog 2026-05-29 amendment: v2.1 tracing-Layer redactor M-T1 ratified (REQ-V2-1-TRACING-LAYER-REDACTOR-001 close-of-pass-3-deferred-half) | accepted   | 2026-05-10 |
| 0020  | Chart buy/sell emphasis (v1.9 Q1–Q9)                | accepted   | 2026-05-10 |
| 0021  | RustQuant adopted as helper, not foundation         | accepted   | 2026-04-17 |
| 0022  | Cost telemetry lives in dedicated `cost` crate      | accepted   | 2026-04-17 |
| 0023  | iced is the single UI stack                         | accepted   | 2026-04-17 |
| 0024  | Audit ledger: raw `sqlx` + SQLite, not `sqlx-ledger`| accepted   | 2026-04-19 |
| 0025  | v0 hand-rolled Binance WS behind trait              | accepted   | 2026-04-17 |
| 0026  | v0 simple paper engine; LOB deferred                | accepted   | 2026-04-17 |
| 0027  | v2.5 — Kronos foundation-model forecast overlay (ONNX + tract) | accepted | 2026-05-16 |
| 0028  | v2.5 — DL forecast overlay trained in `candle` (supersedes 0027) | accepted | 2026-05-16 |
| 0029  | v2.5 — Forecast-checkpoint provenance schema + LFS-anchor strategy | accepted | 2026-05-17 |
| 0030  | Cockpit calls the backtest engine in-process via tightened API | accepted | 2026-05-17 |
| 0031  | `AuditTick<Event, Context>` consumer envelope for audit ledger read path | proposed | 2026-05-17 |
| 0032  | Backtest real-Binance-data path + REVISION.toml data-revision pin | accepted | 2026-05-18 |
| 0033  | v2.5 TCN alpha-investigation report shape + F-verdict algorithm | accepted | 2026-05-18 |
| 0034  | Cockpit training control — audit-DB-as-seam, subprocess lifecycle, R6 in-panel curves | accepted | 2026-05-19 |
| 0035  | Post-training σ_train recalibration via metadata overlay (v2.5 cross-phase contract) | accepted | 2026-05-21 |
| 0036  | PatchTST training contract — patch-embed shape, σ_train post-training, candle attention determinism gate, cost tripwire (v2.5a) | proposed | 2026-05-21 |
| 0037  | Phase B scenario-dispatch extraction — renumbered 0035→0037 to resolve number collision with ADR-0035-tcn-sigma-train-recalibration (audit-2026-05-22) | accepted | 2026-05-19 (number reassigned 2026-05-22) |
| 0038  | v3 vol-forecast V-verdict report shape + GARCH(1,1) baseline contract (parallel to ADR-0033, not extension) | accepted | 2026-05-22 |
| 0039  | LLM-forecaster verdict criteria L0-L4 (parallel to ADR-0033 § D3 and ADR-0038 § D1, not extension) | accepted | 2026-05-22 |
| 0040  | Yahoo realdata path + revision pin (Lab dispatch source) — generalises ADR-0032 to a second data source; engine stays source-agnostic per Q1=(b); 34/34 anchors byte-identical | accepted | 2026-05-24 |
| 0041  | Trader crate split — reflection-memory consumer moves out of strategy into new `crates/trader/`; structurally enforces R8.1 / R10.8 layering invariant; pure package-level refactor (additive-zero anchors) | accepted | 2026-05-26 |
| 0042  | Cockpit activity broadcast — 10th `EventBus` channel + RAII `ActivityHandle` for in-flight-work tape (extends ADR-0012); Q1=(a) broadcast-bus over (b) tracing-layer / (c) per-source polling; capacity-256 lossy ring; 100 ms producer-side throttle; in-memory only (audit ledger remains source of truth); 34/34 anchors byte-identical | accepted | 2026-05-26 |
| 0043  | Simulated network latency + order-book slippage in backtest — D1 always-on code path with default-zero noop (rejects Cargo feature flag); D2 seeded `ChaCha20Rng` sub-stream keyed on `(scenario_seed, order_id)` for replay determinism; D3 linear bps slippage at v0.1.0, defer square-root to v0.2.0; D4 NEW `AuditEvent::SimulatedExecMetrics` variant with skip-when-zero guard; D5 backtest-only scope (live mode untouched); 34/34 anchors byte-identical at v0.1.0 ship via R-NR.1 hard gate | accepted | 2026-05-26 |
| 0044  | Activity-aggregator producer pattern — audit-ledger-writes producer with 100 ms aggregation envelope (extends ADR-0042); D1 placement at `crates/agent/src/activity_audit_aggregator.rs` sibling of `activity.rs` (producer cohesion) over `crates/ui/` (UI is subscriber) / `crates/audit/` (R-NR.1 zero-change contract); D2 internal shape (broadcast::Receiver + AtomicU32 + tokio::time::interval + long-lived ActivityHandle); D3 100 ms cadence verbatim from ADR-0042 § D1.4 (status-bar render budget unchanged); D4 separate-handle Failed emission (NOT main-handle fail() — don't taint successful writes red); D5 idle-end semantics (long-lived handle; drop on first empty window for free "audit active / quiet" boolean); 34/34 anchors byte-identical by construction | accepted | 2026-05-26 |
| 0045  | v5 canonical-config + noop-baseline namespace strategy (v5 v0.2.0 anchor migration) — D1 medium config `LatencySlippageSimConfig { latency_ms_min: 30, latency_ms_max: 80, slippage_bps: 8 }`; D2 two-namespace co-existence (`noop-baseline` historical oracle + `v5-realdata-medium-2026-05` canonical pin); D3 K1-surprise per-scenario flag for operator review; D4 mandatory cross-feature e2e re-check; D5 Sharpe-delta table as permanent regression gate | accepted | 2026-05-27 |
| 0046  | Cockpit toast queue v0.1.0 — `VecDeque<ToastEntry>` cap 5 drop-oldest FIFO; stacked Lumen-card overlay in bottom-right via `iced::widget::Stack` above the 24 px activity tape; dual dismissal (shared 500 ms `ToastDismissRecipe` auto-timeout 5 s + per-card `×` button); clock injection via `Message::ToastTick(Instant)` (no AppState clock field); severity tokens map to existing Lumen palette (FG_2 / UP_500 / INFO_400 / DOWN_500) — zero new tokens; back-compat `toast_message()` method shim retained one cycle; 69/69 anchors byte-identical (UI-only) | accepted | 2026-05-27 |
| 0047  | v5 v0.3.0 full-path wiring + namespace-aware Rust resolver — D1 K2-REACHABLE-CHEAP verdict (~5 LoC `--force-synthetic-bars` CLI flag); D2 per-path plumbing contract for 6 unwired strategies (SmaComposed, TcnOverlay, PatchTstOverlay, Pairs, VolTargetOverlay, GarchVolOverlay; ThresholdSweep deferred — no equity surface); D3 namespace-aware Rust resolver in `crates/reports/tests/strategy_anchors_unchanged.rs` (mirrors `verify_anchors.sh` v0.2.0 pattern); D4 conditional Q1 re-emission (operator chose route (a) revert-to-synthetic); D5 anchor namespace extend `v5-realdata-medium-2026-05` (Q3=(a) same pin); D6 e2e inventory unchanged (3 files) | accepted | 2026-05-27 |
| 0048  | lab-recipe-test-harness — Pattern (d) Combination: Surface 1 boundary-test of `spawn_lab_run` with `MockLabYahooBarSource` (catches sentinel emission + tokio::select! channel survival) + Surface 2 Stop-button gating state-machine test against `model.lab_run_inflight` (catches predicate-gated UI elements); D2 file:line locations at crates/ui/tests/{spawn_lab_run_yahoo_harness,lab_stop_button_gating}.rs; D3 scope 3 regression categories A/B/C; D4 visual regressions NOT caught; D5 per-feature M-FINAL gate for any UI Recipe touch; D6 anchor-additivity (channel-only events; 70/70 byte-identical). § Changelog 2026-05-29 amendments: v0.2.0 cross-surface extension carries D1-D6 verbatim to 7 additional Recipe surfaces (TrainingLogRecipe S1+S2, ActivityAuditAggregator S1, ServerTimeRecipe S2, ToastDismissRecipe S1+S2, TrailMirrorRecipe S2, ActivityRecipe S1, TrainingPoller S1+S2) + visual-fail-html-reporter v0.1.0 inherits forensic-artifact emission pattern from D6 (HTML alongside PNG triple on FAIL only) + ui-test-harness-viewport-matrix v0.1.0 inherits D1-D6 for 22 in-scope visual-fixture viewports + ui-contrast-asserter v0.1.0 inherits D1-D6 boundary-test shape for WCAG 2.1 contrast assertion (Surface 1 = `contrast_ratio` pure-fn over 83-entry PAIRS table; Surface 2 = `ContrastClass { Body, Equity, OptOut(reason) }` dispatch; MIN_PAIRS=60 floor + reference-vector tests; UI_CONTRAST_MODE=warn|gate env var default warn at v0.1.0 → gate at v0.2.0 after operator confirms 2-week WARN observation; 2 GENUINE WCAG-AA defects surfaced by dry-run audit are signal-of-record, not opt-outs). | accepted | 2026-05-28 |
| 0049  | v3-regime-classifier Markov-switching verdict shape — D1 model class Markov-switching regression with operator-set semantic priors {Bull μ>0 σ²low, Bear μ<0 σ²low, Volatile μ=0 σ²high, Calm μ=0 σ²low} + Baum-Welch refinement; D2 4-state K4 contract option (γ) preserve Chop + APPEND Volatile=3, Calm=4 with EmbeddingV1/V2 escape hatch; D3 dispatcher integration with degenerate CashHoldStrategy for Volatile/Calm (SUPPRESSION not LIQUIDATION); D4 verdict shape V-PASS / V-MARGINAL / V-FAIL sibling to ADR-0033; D5 anchor namespace v3.0.0-regime; D6 K-reg-2 mitigation via max_regime_confidence ≥ 0.70 threshold | accepted | 2026-05-28 |
| 0050  | iced-tokio runtime-context contract and cooperative cancellation primitives — D1 (corrected): rt.spawn() for HTTP/transitive tokio; rt.enter() guard only for K8 timer-construction pattern; D2: CancellationToken + abort(); D3 (amended): HTTP/spawn_blocking path test required (not just timers); D4 (new): HTTP/reqwest MUST use rt.spawn() — rt.enter() insufficient for GaiResolver DNS spawn_blocking; codified on Bug #64 D.1.1 recurrence #3; e2e gates: lab_runner_ticker_e2e.rs + lab_runner_cancel_e2e.rs + lab_runner_http_offexecutor_e2e.rs | accepted | 2026-05-29 |

All architectural decisions are now extracted. Remaining Phase 1A
work: final monolith compression (Changelog) and section-file body
finalisation.

## Changelog
- 2026-05-13 (architect): index initialised. ADRs 0001-0004 land in the
  same session (Session 1).
- 2026-05-13 (architect): ADRs 0005-0007 added (hot-load decisions
  cluster) during Session 4.
- 2026-05-13 (architect): ADRs 0008-0012 added (v0.5 strategy-registry
  resolution cluster) during Sessions 5+6.
- 2026-05-13 (architect): ADR-0013 added (v1 cross-sectional momentum
  resolution cluster) during Session 7.
- 2026-05-13 (architect): ADR-0014 added (v1.5a mean-reversion pairs
  cluster) during Session 8.
- 2026-05-13 (architect): ADRs 0015-0017 added (operator reports,
  real-mtm, v1.5b multi-venue) during Session 9.
- 2026-05-13 (architect): ADRs 0018-0020 added (Lumen Phase 1, v2 LLM,
  chart buy/sell emphasis) during Session 10. All strategy-registry
  Q&A clusters now extracted.
- 2026-05-16 (architect): ADR-0027 added (v2.5 Kronos forecast
  overlay — ONNX + `tract` integration, signal-level overlay on v1
  momentum). Cross-cutting overlay pattern landed in new section
  file `spec/architecture/12-forecast-overlay.md`.
- 2026-05-16 (orchestrator): ADR-0028 supersedes ADR-0027 after Wave A
  bootstrap surfaced (a) Kronos lives outside `transformers`,
  (b) two-model architecture requires Rust-side sampling-loop
  reimplementation, (c) crypto-fit was never validated. Operator
  pivoted v2.5 to "train small custom Transformer/TCN in `candle`".
  Wave A crates (forecast / replay-cache / core forecast types) are
  model-agnostic and preserved; only Kronos-specific files removed.
- 2026-05-13 (architect): ADRs 0021-0026 added (Foundation libraries
  substantive decisions) during Session 11. iced UI body migrated to
  06-ui-and-cockpit.md. All architectural decisions now in ADRs.
- 2026-05-17 (architect): ADR-0029 added (TCN/forecaster checkpoint
  provenance schema + LFS-anchor strategy — cross-phase contract for
  v2.5 TCN, v2.5a PatchTST, v2.5b vanilla Transformer). Also
  backfilled the registry table row for ADR-0028 (previously only in
  changelog).
- 2026-05-17 (architect): ADR-0030 added (cockpit in-process backtest
  engine API). Opens the `ui → backtest` edge for the Lab Run button
  shipped by `ui-rethink-phase-a-lab` per operator-decision Q-A2.
- 2026-05-17 (orchestrator): ADR-0031 added (status: proposed) —
  `AuditTick<Event, Context>` consumer envelope for the audit ledger
  read path. Borrowed from
  [barter-rs](https://github.com/barter-rs/barter-rs) per the survey at
  `spec/dev-notes/external-code-patterns-2026-05-17.md`. Decouples
  consumer-side state replicas from producer's channel choice via
  generic Iterator. Strictly additive (existing taps + broadcast tee
  coexist); zero hot-path impact; 11 anchors stay byte-identical.
  Implementation queued in `spec/backlog.md ## Queue`.
  Establishes the invocation pattern reused by Phase B / Phase E
  Compare matrix / v3 continuous-paper.
- 2026-05-18 (architect): ADR-0032 added — backtest real-Binance-data
  path + `REVISION.toml` data-revision pin for the four new
  `top10-*-fy-tcn-overlay[-weights]-realdata` scenarios. Locks
  (a) module placement (`crates/backtest/src/realdata.rs`,
  feature-gated `realdata`, reuses `data::ReplayFeed::merge_symbols`),
  (b) `REVISION.toml` schema + aggregate-SHA algorithm,
  (c) orthogonal `ScenarioDataSource` axis on `Scenario` (not new
  `ScenarioStrategy` variants), (d) `data_revision_sha` in both
  frontmatter (forensics, excluded from body hash) and a new
  `## Data source` body section (anchor integrity, covered by body
  hash). Existing 15 anchors stay byte-identical (K6 + K10).
  Closes T-AR-1, T-AR-3 of
  `spec/backtest-real-binance-data/tasks.md`.
- 2026-05-19 (architect): ADR-0034 added — Cockpit training control.
  Locks (a) audit-DB-as-seam (training events flow `train_tcn` writer
  → SQLite `training_events` → 1 Hz cockpit subscription reader) over
  alternatives (IPC, `agent::EventBus` channel) — Q1=(a) operator-
  confirmed 2026-05-19, (b) additive migration
  `010_training_events.sql` (anchor-byte-safe; 19 anchors stay
  identical), (c) subprocess lifecycle (sibling `lab::trainer.rs` to
  `lab::runner.rs`; SIGKILL-immediate on Cancel; PID column for
  orphan-detect; hard-fail on missing `--audit-db` file;
  `catch_unwind` boundary for `kind='failed'` row survivability),
  (d) R6 in-panel `widgets::training_plot` (new module, NOT a
  `ChartKind::TrainingCurves` extension of the main chart widget),
  (e) K5 CLI-drift mitigation via `cargo run --bin train_tcn -- --help`
  golden-snapshot test (NOT runtime `--print-config-schema`
  validation), (f) Q4=(a) status-strip orphan annotation (no
  auto-route). Surfaces a latent WAL-mode gap in
  `Ledger::open` (the `?mode=rwc` URL does not issue
  `PRAGMA journal_mode = WAL;`) as a follow-up backlog item;
  non-blocking for this feature given the 1-write-per-5-30-min cadence.
  Closes T-AR-3 of `spec/cockpit-training-control/tasks.md`.
- 2026-05-18 (architect): ADR-0033 added — v2.5 TCN alpha-investigation
  report shape + F-verdict algorithm. Locks (a) read-path placement
  (new bin `crates/forecast/src/bin/forecast_distribution.rs`, NOT
  extension of `crates/backtest`'s TCN dispatch — preserves anchor
  neutrality against the 4 byte-locked `-realdata` anchors and
  generalises to v2.5a/v2.5b alpha-investigations), (b) report
  frontmatter-vs-body discipline + floating-point canonicalisation
  (`%.6f` / `%.9f` / `(x * 1e6) as i64` for bin edges) following the
  ADR-0032 § D4 precedent, (c) F-verdict decision algorithm
  (deterministic F1/F2/F3/F4 classifier over (abs_p95, std,
  sigma_train, frac_inside_epsilon, confidence_gate_survival) with
  F-MIXED escape hatch for BS-1 vs. BS-2 disagreement), (d) Sharpe /
  Sortino / Calmar formulas (sqrt(24·365) hourly annualisation; new
  helpers in the M-SHARPE bin, NOT reuse of `compute_sharpe` which is
  minute-annualised). Existing 19 anchors stay byte-identical
  (R6 non-regression contract). Closes T-AR-3 of
  `spec/v25-tcn-alpha-investigation/tasks.md`.
- 2026-05-21 (architect): ADR-0035 added — Post-training σ_train
  recalibration via metadata overlay (cross-phase contract for v2.5
  TCN, v2.5a PatchTST, v2.5b vanilla Transformer). Locks (a) σ_train
  must be derived in a frozen-weights post-training forward pass, NOT
  via in-loop accumulation (the `train_tcn.rs:606,676-678,733-741`
  bug pattern is deprecated), (b) overlay-file convention
  (`.metadata.recalibrated.json` co-located with the original; original
  stays byte-identical), (c) on-disk JSON number convention for
  `sigma_train` (intentional divergence from ADR-0029 § 2 rule 5's
  string-encoded form — load-bearing for `.as_f64()` parity at the
  inference read site), (d) additive `--metadata-path` CLI flag on
  consumers (never auto-prefer overlay), (e) σ_train-not-in-safetensors
  invariant codified as test. Does NOT supersede ADR-0033 § D3
  (F-verdict algorithm stays immutable per `v25-tcn-recalibrate`
  Q4 = (a)). Closes T-AR-2 of
  `spec/v25-tcn-recalibrate/tasks.md`.
- 2026-05-21 (architect, M-T1): ADR-0036 added (status: proposed) —
  PatchTST training contract for v2.5a phase-2 ship. Locks (D1) PatchTST
  layer skeleton (patch embed + learnable PE + pre-LN transformer
  encoder + projection head; channel-independence per Nie et al § 3.2;
  PatchTST/42 small config = ~410k params), (D2) ADR-0029 canonical-arch
  descriptor extended additively with PatchTST architecture fields
  (model_family, patch_len, stride, d_model, n_heads, d_ff, n_layers,
  dropout, context_len) + tokenisation.target_horizon_bars; existing
  TCN model_revision SHAs unchanged, (D3) σ_train post-training
  derivation per ADR-0035 § D1 cross-phase contract verbatim — no
  in-loop accumulator; architect's grep-based code-review check codified,
  (D4) cost tripwire (24h hard limit + 3× median multiple) with
  continue-on-fire + escalation policy, (D5) K2 candle-attention
  determinism gate (CPU byte-identity + Metal-vs-CPU drift < 1e-4 per
  ADR-0029 § 4), (D6) anchor strategy under version `v2.5a.0-patchtst`
  (2 anchors additively; 28 predecessor anchors byte-immutable),
  (D7) sibling strategy `patchtst_overlay_momentum.rs` (NOT a model-
  agnostic refactor of `tcn_overlay_momentum.rs` — K6 scope-creep
  guard). Sibling deliverable: `spec/v25a-patchtst-overlay/decomp.md`.
  Closes T-AR-2 of `spec/v25a-patchtst-overlay/tasks.md`.
- 2026-05-24 (architect, M-T1): ADR-0040 added — Yahoo realdata path
- 2026-05-26 (architect, M-T1): ADR-0041 added — trader crate split;
  recovery brief for `reflection-memory-trader-wiring` v0.1.0 (P0
  gate-test red on `main`); locks Q1=(a) new `crates/trader/` +
  Q2=(a) clean-cut move + Q3=(a) inverse-API; corrects analyst
  file-count miscount (9 files / 10 test suites, not 8 / 13).
- 2026-05-26 (architect, M-T1): ADR-0043 added — simulated network
  latency + order-book slippage in backtest; default-zero noop preserves
  34 anchors via R-NR.1 hard gate; v0.2.0 anchor-migration brief
  deferred per Q5.
- 2026-05-26 (architect, M-T1): ADR-0044 added — activity-aggregator
  producer pattern (extends ADR-0042). Codifies the
  broadcast-receiver + `AtomicU32` + `tokio::time::interval` +
  long-lived `ActivityHandle` with idle-end semantics recipe as
  reusable for future high-frequency event sources. Locks D1
  placement at `crates/agent/src/activity_audit_aggregator.rs`
  (producer cohesion with existing `activity.rs`), D2 internal
  shape (50 ns/tick fetch_add hot path), D3 100 ms cadence verbatim
  from ADR-0042 § D1.4, D4 separate-handle Failed emission (NOT
  main-handle `fail()` — don't taint successful writes red), D5
  idle-end semantics. Q1=(b) per-window 100 ms / Q2=(a) redacted
  "Audit: N writes" / Q3=(a) sibling Failed event all locked at
  analyst-recommended defaults under standing Autoapprove.
  Zero changes to `crates/audit/` (R-NR.1 anchor-additive
  contract); 34/34 anchors stay byte-identical. Closes T-AR-4 of
  `spec/cockpit-activity-audit-ledger-producer/tasks.md`.
- 2026-05-26 (architect, post-ship): ADR-0042 added — cockpit activity
  broadcast; codifies the design shipped at
  `cockpit-activity-status-bar v0.1.0` (commits `4248c00` + `ea52057`
  + `49bf342` + `ef6f018` + `0ff402f` + `f728334`). 10th `EventBus`
  channel (`activity_tx`, capacity 256) + RAII `ActivityHandle` with
  panic-aware `Drop` + 100 ms producer-side throttle. Q1=(a)
  broadcast-bus pattern wins over (b) tracing-layer (fragile + slow,
  ties UI to log filtering) and (c) per-source polling (coupling-heavy,
  doesn't extend). Lossiness OK because activity tape is operator-
  eyeball UX, not audit (R-NR.4 in-memory only; audit ledger remains
  source of truth). Deferred to v0.1.1: PII redaction for `LlmCall`
  producer (K4); aggregator design for `AuditLedgerWrite` producer
  (R5.2 / K3). 34/34 anchors stay byte-identical. Closes T-AR-4 of
  `spec/cockpit-activity-status-bar/tasks.md`.
  + revision pin (Lab dispatch source). Generalises ADR-0032's
  revision-pin protocol to a second data source (Yahoo Finance) on
  the Lab dispatch path. Locks D1 (module placement —
  `crates/data/src/yahoo.rs` feature-gated `yahoo`; CLI binary
  co-located at `crates/data/src/bin/fetch_yahoo_klines.rs`), D2
  (external dep `yahoo_finance_api 4.1.x` with CLAUDE.md 6-item
  library-compat checklist all green), D3 (revision-pin protocol —
  cadence subdir on disk + per-fetch `[revision.yahoo_response]`
  table for K2 forensics; aggregate-SHA algorithm verbatim-shared
  with ADR-0032), D4 (engine remains source-agnostic per Q1=(b);
  Lab swaps bars upstream via existing `bars_override` hook; the 4
  cross-sectional arms reject `data_source = YahooCache` with typed
  error), D5 (`YahooBarSource` API surface with `load_cached`
  /`fetch_and_cache` split feature-gated so cockpit doesn't pull
  tokio into iced — K10 mitigation), D6 (adaptive cadence Q4=(c) —
  `Interval::derive_from_range` 10-row truth table), D7 (boundary
  ticker conversion Q6=(a) — `binance_to_yahoo_ticker` 10-entry
  table in `crates/data/src/yahoo.rs` for CLI reuse), D8
  (`Venue::Yahoo` variant cascade — additive; existing rows
  untouched; clippy `-D warnings` drives exhaustive-match
  completion). 34/34 anchors stay byte-identical (anchored body-SHAs
  all originate from CLI `--features realdata` paths that construct
  `ScenarioConfig` without the new fields; `#[serde(default)]`
  preserves byte-identity). Closes T-AR3 + T-AR6 of
  `spec/lab-yahoo-realdata/tasks.md`.

- 2026-05-22 (architect, M-T1): ADR-0039 added — LLM-forecaster verdict
  criteria L0-L4 (parallel to ADR-0033 § D3 and ADR-0038 § D1, NOT
  extension; PARALLEL is the operator-locked precedent per
  retrospective lesson #2 + Q6 operator-pick 2026-05-22). Locks (D1)
  L0-L4 priority tree (L1 bias collapse `hold_frac ≥ 0.95` / L2
  calibration failure `|confidence_outcome_corr| < 0.05` / L3 cost
  overrun `overrun_ratio > 2.0 OR cost_actual > cost_cap` / L4
  reasoning trace degenerate `short_frac > 0.50 OR duplicate_frac >
  0.50` / L0 PASS routes to L_ALPHA gate) — analyst-strawman LOCKED
  per Q6 operator constraint with architect cap "≤2 new priorities
  beyond strawman before re-surface" codified inline at D1.b last
  paragraph, (D1.c) L_ALPHA strategy-side gate (Sharpe-delta
  thresholds inherited from ADR-0038 § D1.c verbatim: L-ALPHA-UNLOCKED
  `≥ +0.10` / L-MARGINAL `[+0.05, +0.10)` / L-NO-ALPHA `< +0.05`) +
  5-cell joint advisory verdict routing table, (D2) verdict section
  shape (frontmatter-vs-body discipline per ADR-0032 § D4 + ADR-0033
  § D2 + ADR-0038 § D2; full body layout delegated to
  `spec/v3-llm-forecaster/decomp.md` § T-AR-6), (D3) L_ALPHA Sharpe-
  comparison bin extension (additive `--scenario llm-forecaster-bs1`
  dispatch arm on `crates/forecast/src/bin/sharpe_comparison.rs`),
  (D4) replay-cache namespace additive extension (dedicated
  `data/llm-forecaster-replay.db` + checked-in compressed fixture <
  50 MB per K5; reuses `crates/llm::RecordingProvider/ReplayProvider`
  schema verbatim), (D5) strategy-side + Phase F Assistant slot
  composition v0.1.0 with Q4=(b) overlay deferred to v0.1.1 and
  Q4=(d) all-three-as-builders deferred to v0.2.0+, (D6) anchor +
  version naming (new `v3.0.0-llm-forecaster` namespace; +2 anchors
  at developer Wave G close; sharpe-comparison NOT anchored at
  v0.1.0; re-emission protocol inherited from ADR-0038 § D6.b
  verbatim). Closes T-AR-9 of
  `spec/v3-llm-forecaster/tasks.md`.
