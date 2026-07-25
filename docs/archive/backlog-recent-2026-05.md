# Backlog — Recent (shipped) archive, through 2026-06-08

> Extracted verbatim (links flattened to plain text) from `spec/backlog.md`
> `## Recent (shipped)` on 2026-06-11 per `CLEANUP-PLAN.md` P2-3.
> Provenance: every entry remains in git history of `spec/backlog.md`.

## Recent (shipped)

### 2026-05-29 cohort

- **v3-regime-classifier v0.1.0** — **RETIRED 2026-05-29** with
  empirical T-REG-NO-ALPHA verdict (net Sharpe-delta -0.294113 vs
  un-overlaid v1 momentum on 2024 held-out validation) + V-REG-5
  (classifier fails to separate regimes meaningfully). 5 dev waves
  shipped over 2 days under the new durable contract: A core
  Markov-switching (14 tests) + B K4 RegimeTag extension + C
  dispatcher with cash-fallback + D audit RegimeTag + Trail UI
  column + E 4 anchored backtest reports. **Zero MIGRATION:
  comments across all 5 waves** — durable contract validated
  end-to-end; gates correctly identified the strategy doesn't work
  without accumulating debt. Operator R-O 2026-05-29 picked Option 1:
  retire + close v3 three-pick set. **v3 three-pick scorecard
  CLOSED**: C1 retired 2026-05-22 (-0.022); C2 (this) retired
  2026-05-29 (-0.294); C5 shipped v0.1.0-PARTIAL (inconclusive
  Sharpe). v3 strategy reformulation program empirically dead;
  anchored Wave E bodies (75/75 total) stay as scientific record
  per ADR-0038 § D6.a. Architect artifact: ADR-0049 (5-D contract
  + Markov-switching priors + K4 γ-encoding + dispatcher cash-fallback
  + V-REG/T-REG shape + namespace pin). Commits: `0e75c45`,
  `6b47027`, `8252021`, `053b2e8`, `9d867c1`, `2362ed2`, `5eebe4b`,
  `ced662d`, `cd073c0`. See
  `spec/v3-regime-classifier/feature.md`
  for shipped_disposition.

- **lab-yahoo-realdata-v0.1.3-rev-frontmatter-and-binance-eth-h1 v0.1.0** —
  shipped 2026-05-29 (operator-approved). Closes the 2 architect-flagged
  design notes from v0.1.2's M-FINAL via the durable-over-quick choices:
  (1) **REVISION.toml `rev=` body→frontmatter migration** through a
  canonical Yahoo report-emit helper at
  `crates/backtest/src/report/yahoo.rs` (Q1=(a) durable — future
  MACD/RSI/BBands emitters inherit byte-identically). Helper-bypass
  regression guard at `crates/backtest/tests/yahoo_report_helper_shape.rs`
  (3 grep assertions) is the durable-boundary contract. (2) **Binance
  ETH H1 scenario** `eth-2024-h1-sma-cross` registered with 17,543
  hourly bars (real Binance parquet auto-detect); retires v0.1.2's
  Yahoo-to-Yahoo K1 fallback. **H1 PASS DIRECT at 6.78%** (Yahoo ETH
  daily +2.76% vs Binance ETH hourly +9.54%); well under 30% threshold.
  Anchor cascade **70 → 71**: row 69 BTC SHA `076929bb...` in-place
  under existing namespace `lab-yahoo-realdata-v0.1.1` (Q2=(a)); row
  70 ETH daily byte-identical (`e59a5f87...`); row 71 NEW
  `eth-2024-h1-sma-cross` SHA `bd4001e4...` under new namespace
  `lab-yahoo-realdata-v0.1.3`. ETH H1 determinism 4/4 (2 dev + 2
  tester runs). 33 Binance SMA anchors byte-identical via None-arm
  of `report::sma::write` (None arm contract). 93 backtest tests
  PASS. ADR-0040 § Changelog amendment (no new ADR). Carve-outs
  explicitly owned in deck: ETH daily row 70 bundled with v0.1.4
  BNB bulk re-emit; trace-broken-path × 4 spec-lint hits owned by
  parallel v3-regime-classifier Wave E. Commits: `6f97a40`,
  `5eebe4b`, `e74204a`, `29c1aef`, `72b3947`. See
  `spec/lab-yahoo-realdata-v0.1.3-rev-frontmatter-and-binance-eth-h1/feature.md`.

### 2026-05-28 cohort

- **v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit v0.1.0** —
  shipped 2026-05-28 (operator-approved). Closes the v0.3.0 SOFT-PASS
  carve-out: 8 candle/realdata-feature-gated scenarios re-emitted under
  canonical `LatencySlippageSimConfig { 30 ms / 80 ms / 8 bps }`; 8
  SHAs overwritten in-place under namespace `v5-realdata-medium-2026-05`
  (anchor count stays 70/70). **Compound determinism (candle × realdata
  × friction) gate DISCHARGED**: dev's 2-run byte-identity gate PASS
  (8/8) + tester's independent witness on 2 of 8 (PatchTST + Vol-target)
  MATCH. ADR-0047 carries forward unchanged; no new ADR. Friction-real
  scenario fleet grew 11 → 19 of 70 anchors. Sharpe-delta highlights:
  TCN-realdata Δ -$36.5k / -$29.8k (5× trade-frequency amplification);
  PatchTST Δ -$25.2k (H2 FALSIFIED — fewer trades than TCN);
  Vol-target GARCH Δ -$9.5k (17% gross-fill reduction). 0 K1 surprises
  across all 8. Commits: `f9eb683`, `dcb1935`, `d8fe484`, `57d67cd`,
  `ed00e65`. See
  `spec/v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit/feature.md`.

- **lab-recipe-test-harness v0.1.0** — shipped 2026-05-28 (operator-
  approved). P1 tooling investment closing the channel/subscription
  test gap exposed by the Bug #64 D.1.1+D.2.1 revert (commit
  `05937e4`). Architect pattern (d) Combination: Surface 1 boundary-
  test for `spawn_lab_run` with `MockLabYahooBarSource` + Surface 2
  Stop-button gating state-machine test against `model.lab_run_inflight`.
  New `pub trait LabYahooBarSource` extraction in `crates/ui/src/lab/runner.rs:194-260`;
  `Box<dyn>` for ergonomic test construction; production path
  backwards-compatible via `None` injection. 6 new tests across 2 new
  files (3 in `spawn_lab_run_yahoo_harness.rs` + 3 in
  `lab_stop_button_gating.rs`). **T-T4 falsification CONFIRMED**:
  tester independently commented out `state.rs:2147` and verified 2
  Surface 2 tests fail at `lab_stop_button_gating.rs:133` + `:182`;
  restore verified 3/3 PASS. Zero anchor delta (channel-only events,
  no file output); 70/70 byte-identical. K5 regression intact (5/5).
  411 lib tests PASS; clippy 0 new (9 pre-existing). Unblocks AND
  gates the Bug #64 D.1.1+D.2.1 re-attempt. Future UI Recipe touches
  can opt into the same harness pattern. Commits: `a971008`, `648d470`,
  `dbe1609`, `aaa5bc9`. ADR-0048. See
  `spec/lab-recipe-test-harness/feature.md`.

- **lab-yahoo-realdata-v0.1.2-eth-usd-anchor-and-cache-badge v0.1.0** —
  shipped 2026-05-28 (operator-approved). Closes Q1 + Q3 of v0.1.1
  presenter deck's open list. Anchor count **69 → 70**:
  `eth-yahoo-2024-1d-sma-cross` locked under namespace
  `lab-yahoo-realdata-v0.1.2` (SHA `e59a5f87daf0cc58ce8be2e1695dfc2c
  cc3ab76bd976b54c957e9e3c5ed4199a`). `run_yahoo_sma.rs` extended with
  `--ticker <TICKER>` Clap arg (default BTC-USD; scales DRY across the
  remaining 8 crypto-mirror tickers; `ALLOWED_YAHOO_TICKERS` 10-row
  validation surface). NEW aggregate cache-state SUMMARY badge widget
  (`cache_state_summary_badge`) in a NEW Lab tab toolbar row (operator
  Q2 override) — "Yahoo cache: N tickers · last fetch YYYY-MM-DD".
  Cached on `LabState::cache_summary` with invalidation hooks in
  `LabSelectDataSource` + `LabRunCompleted` per ADR-0040 § Changelog
  D-V0.1.2-1. Two-lane parallel ship (M-DEV backtest + M-DEV-UI; zero
  file overlap). H1 PASS at 0.84% via K1 synthetic fallback (Yahoo
  ETH vs Yahoo BTC same-window); H2 ×5 determinism PASS. UI lib 411
  PASS (+14); panel snapshots 90 PASS (+4); cross-feature canary
  (cockpit_training_pressed_wiring) 5/5 PASS. **SOFT-PASS** qualifier:
  BTC body SHA drifted from `8045623b...` to `d2a709ef...` because
  `REVISION.toml` aggregate changed when ETH-USD was fetched
  (`rev=` line in report body). Verify_anchors.sh still resolves
  70/70 correctly per ADR-0038 § D6 byte-immutability via on-disk
  file. Pattern flagged for v0.1.3 architect attention. Commits:
  `cf7015c`, `d4c4c45`, `bd7e04b`, `9638ff8`, `1fd72b7`. ADR-0040
  Changelog. See
  `spec/lab-yahoo-realdata-v0.1.2-eth-usd-anchor-and-cache-badge/feature.md`.

- **cockpit-toast-queue v0.2.0 cleanup v0.1.0** — shipped 2026-05-28
  (operator-approved). Closes the v0.1.0 ship's architecture-deviation
  footnote: retires the `pub toast_message: Option<SmolStr>` FIELD,
  the `toast_message()` METHOD shim, and the 2-line stale comment in
  `cockpit_live.rs:1181-1182` — all eliminated. Post-cleanup
  `grep -rn "toast_message" crates/` → **0 matches** anywhere.
  Sub-route (b) FULL REMOVAL chosen (analyst recommendation aligned;
  audit confirmed only test code referenced the method shim). 2 test
  field-WRITE sites migrated to `Message::ShowToastWithSeverity` /
  `Message::ShowToast` dispatch (mirrors production `cockpit_live.rs`
  pattern); 5 field-READ assertions flipped to direct
  `toast_queue.front()` access. K5 regression 5/5 PASS; v0.1.0
  integration 4/4 PASS; 397 ui lib tests PASS; 69/69 anchors
  byte-identical (UI-only). ADR-0046 § T-AR-5 one-cycle migration
  commitment honored. Commits: `8ebc12a`, `2dcb112`, `8c074bd`. See
  `spec/cockpit-toast-queue-v0.2.0-cleanup/feature.md`.

### 2026-05-27 cohort

- **v5-latency-slippage-sim-v0.3.0-full-path-wiring v0.1.0** —
  shipped 2026-05-27 (operator-approved). Closes v0.2.0's accepted
  scope gap: friction-real anchored scenarios **2 → 11** (was 2 of 34
  at v0.2.0; momentum-only). `LatencySlippageSimConfig` now plumbs
  through 6 strategy paths via new `crates/backtest/src/scenarios/sim.rs`
  shared helper (lifted from momentum.rs as anchor-additive per
  ADR-0038 § D6.a). New `--force-synthetic-bars` CLI flag (~5 LoC)
  honours operator Q1=(a) revert-to-synthetic for Group A SMA/Composed
  re-emission — preserves friction-free oracle for all 69 anchors;
  v0.2.0's Group A canonical SHAs become stranded artifacts. t1937
  test refactored to namespace-aware resolver (Namespace::Noop /
  Namespace::Canonical) — mirrors `verify_anchors.sh` v0.2.0 pattern;
  future-proof against subsequent canonical re-emissions. 11
  canonical reports re-emitted; 9 SHAs overwritten in `spec/anchors.toml`
  in-place under same canonical namespace pin `v5-realdata-medium-2026-05`
  (Q3=(a)). **0 K1 surprises** across all 11 re-emitted scenarios.
  Cross-feature e2e tests all PASS (latency_slippage_sim_e2e 3/3,
  vol_targeting 1/1, vol_killswitch 4/4). **SOFT-PASS qualifier**:
  8 candle/realdata-feature-gated scenarios deferred to v0.4.0
  (plumbing wired; feature-flagged rebuild needed): TCN-weights × 2,
  TCN-realdata × 4, PatchTST × 1, VolTarget-GARCH × 1. Commits:
  `1267d39`, `4fd1095`, `275a6d0`, `21bda41`, `61db5f9`, `fe6b14a`.
  ADR-0047. See
  `spec/v5-latency-slippage-sim-v0.3.0-full-path-wiring/feature.md`.

- **vol-killswitch-overlay-noop-fix v0.1.0 (Bug #65 — P0 safety
  wiring-bug recovery)** — fix landed 2026-05-26 in tree; **spec
  retroactively closed 2026-05-27** by orchestrator during the
  audit-2026-05-27 P0 triage after the auditor flagged the
  paperwork drift (feature.md was `status: draft`, trace.toml was
  `state: proposed`, but bug-log #65 records FIXED 2026-05-26 with
  Q4=(p3) "Both" — fix test fixture AND broaden overlay filter).
  Verified at retroactive close: `cargo test -p strategy --test
  vol_killswitch_overlay_end_to_end` → 4/4 PASS. No formal
  test-final/<DATE>.md was authored at original ship; bug-log #65
  entry is the authoritative shipping record (precedent for
  Bug-fix briefs whose scope makes a full test-final overkill).
  **Trace**: `REQ-VOL-KILLSWITCH-NOOP-FIX-001` flipped
  `proposed → passed`. **Bug log**: `spec/bug-log.md` § #65.

- **cockpit-toast-queue v0.1.0** — shipped 2026-05-27 (operator-approved).
  Replaces the cockpit's single-slot toast REPLACE semantic with a
  bounded multi-toast queue. `VecDeque<ToastEntry>` capped at 5 with
  drop-oldest FIFO; stacked Lumen-card overlay in the bottom-right via
  `iced::widget::Stack`; 5 s auto-dismiss via shared 500 ms
  `ToastDismissRecipe` (6th cockpit subscription) + per-card `×` button.
  Severity tokens reuse existing Lumen palette (`FG_2 / UP_500 /
  INFO_400 / DOWN_500`) — zero new design tokens. K5 back-compat shim
  keeps `cockpit_training_pressed_wiring` regression 5/5 green.
  Architecture deviation flagged: `pub toast_message: Option<SmolStr>`
  FIELD kept alongside queue + method shim (dead-store relative to
  queue; annotated `// MIGRATION: remove at v0.2.0`). 4 integration +
  4 unit + 86 panel-snapshot tests PASS; 69/69 anchors byte-identical
  (UI-only). Operator-side smoke tests T-D-N16/T-D-N17 deferred per
  AGENT.md human-verification recipe contract. Commits: `8480ded`,
  `9cf813a`, `a723d24`, `896baab`. ADR-0046. See
  `spec/cockpit-toast-queue/feature.md`.

- **lab-yahoo-realdata v0.1.1 (live-cache + Yahoo anchor lock)** —
  shipped 2026-05-27 (operator-approved follow-up to v0.1.0). First
  Yahoo Finance anchor locked: BTC-USD 2024 1d SMA cross.
  Operator-populated cache at `data/yahoo/BTC-USD/1d/2024/` (12
  parquets, 366 bars, REVISION.toml SHA `7b33166e1eb8...`). New
  `crates/backtest/src/bin/run_yahoo_sma.rs` binary (247 LoC, gated by
  `yahoo` feature). Anchor count 68 → 69; new row
  `btc-yahoo-2024-1d-sma-cross` under namespace
  `lab-yahoo-realdata-v0.1.1`. **H1 PASS** at 9.03% Yahoo-vs-Binance
  equity divergence (well below 30% threshold). **H2 PASS** at 100%
  fetch success (trivially satisfied at scale=1). Determinism
  confirmed via 2 independent re-runs of the new binary. Tester
  formal FAIL on workspace fmt + gallery test was *external* — both
  blockers attributable to in-flight cockpit-toast-queue dev; resolved
  by toast-queue landing. v0.1.2 follow-on: T-D2 cache-state badge UI,
  multi-ticker fetch, T-T5 cockpit-smoke, T-T8 idle-CPU. Commits:
  `bb14e11`, `8bd6b5c`, `9cf813a`, `a723d24`. See
  `spec/lab-yahoo-realdata/feature.md`.

- **cockpit-activity-audit-ledger-producer v0.1.0** — shipped 2026-05-27
  (operator-approved). Closes the activity-tape producer trio (LLM + Training
  + audit-ledger). New `crates/agent/src/activity_audit_aggregator.rs` (~210
  LoC) subscribes to existing `crates/audit/src/tick.rs` `AuditTick<AuditEvent>`
  broadcast — ZERO changes to `crates/audit/`. 100 ms time-window envelope
  (Q1=(b)); PII-redacted `"Audit: N writes"` label (Q2=(a)); separate-handle
  Failed-event emission (Q3=(a)). Long-lived `ActivityHandle` with idle-end
  semantics. Criterion benches: counter increment 1.797 ns / fan-out 46.81 ns
  / idle-end 131.98 ns / K3-discharge anchor-replay parity 0.12 % < 1 % budget.
  6/6 audit-ledger tests + 2/2 UI tests PASS (1 ignored by K5 design).
  Anchor-additivity preserved by construction (zero crates/backtest|strategy|
  audit/journal|exec|cost changes). M-FINAL FAIL on 3 housekeeping issues
  (fmt + 1 clippy + 1 frontmatter status) inline-fixed by orchestrator at
  commit 6b494aa; tester K3-collision-note in addendum is informational
  (resolves at v5 v0.2.0 Wave B namespace-aware verify_anchors). Commits:
  `8b67669`, `6b494aa`. ADR-0044. See
  `spec/cockpit-activity-audit-ledger-producer/feature.md`.

- **v5-latency-slippage-sim-v0.2.0-anchor-migration v0.1.0** — shipped
  2026-05-27 (operator-approved Ship Route (a) — partial migration accepted).
  Anchor count doubled 34→68: 34 noop-baseline rows preserve original SHAs
  as friction-free oracle; 34 canonical rows under namespace pin
  `v5-realdata-medium-2026-05` carry re-emitted SHAs under
  `LatencySlippageSimConfig { latency_ms_min: 30, latency_ms_max: 80,
  slippage_bps: 8 }`. `scripts/verify_anchors.sh` extended namespace-aware
  (T-AR-3 step 5 escape hatch invoked). Sharpe-delta table at
  `spec/v5-latency-slippage-sim-v0.2.0-anchor-migration/reports/sharpe-delta-table-2026-05-27.md`
  documents 8 scenario groups: only Group B (top-10 momentum, 2/34 scenarios)
  received real v5-sim migration (Δequity -$3.5k to -$5.4k); Group A
  (5x SMA/Composed) Δequity +$48k to +$83k driven by synthetic→real-Binance
  data-source auto-switch, NOT v5 sim; Groups C-F (12 scenarios — Pairs /
  TCN / PatchTST / VolTarget) canonical = noop SHA byte-identical (sim
  not wired into those construction sites); Groups G-H (15 analysis /
  success reports) no equity metrics. **0 K1 surprises** across all 34
  scenarios. Cross-feature e2e tests 8/8 PASS (latency_slippage_sim_e2e
  3/3 + vol_targeting 1/1 + vol_killswitch 4/4). Operator-accepted scope
  gap → v0.3.0 Queue row covers (a) wire LatencySlippageSimConfig into the
  6 remaining strategy paths; (b) operator-decide for Group A re-anchor
  (revert to synthetic OR accept real-Binance baseline); (c) refresh
  `t1937_nine_strategy_anchors_unchanged` test resolver. Commits:
  `d2cc343`, `c223d11`, `4dfa2d8`, `d191227`. ADR-0045. See
  `spec/v5-latency-slippage-sim-v0.2.0-anchor-migration/feature.md`.

- **v5-latency-slippage-sim v0.1.0** — shipped 2026-05-27 (operator-approved
  triple-batch). Deterministic latency + slippage simulator in `crates/exec`
  + `crates/cost`. Default-zero noop preserves all 34 anchors byte-identically;
  CLAUDE.md non-negotiable overlay-e2e divergence test (R5) shipped from day 1
  (3/3 PASS). Murmur3-style finalizer keyed on `(scenario_seed, order_id)`
  (D2 deviation accepted via ADR-0043 Changelog amendment — ChaCha20Rng
  replaced for hot-path perf). Criterion baselines: `apply_latency_noop`
  2.35 ns, `apply_latency_jitter` 2.50 ns, `apply_slippage_10bps` 22.7 ns,
  `noop_8760_fills` 73.9 µs, `enabled_8760_fills` 171.6 µs. New
  `AuditEvent::SimulatedExecMetrics` variant with skip-when-zero guard.
  v0.2.0 anchor-migration brief at
  `spec/v5-latency-slippage-sim-v0.2.0-anchor-migration/`
  picks the canonical non-zero friction profile (operator-decide Q1-Q4
  pending). Commits: `a5f8647`, `c46fd45`. ADR-0043. See
  `spec/v5-latency-slippage-sim/feature.md`.

- **cockpit-activity-llm-producer v0.1.0** — shipped 2026-05-27 (operator-
  approved triple-batch). v0.1.1 follow-on of `cockpit-activity-status-bar
  v0.1.0` closing the parent's Q8 forward-list. `ActivityHandle` wired
  around `crates/trader/src/llm_forecaster/anthropic_impl.rs:412-516`
  via new `with_activity_sender()` builder setter. PII-redacted label
  `"LLM call: <model_id>"` enforced at producer boundary (no prompt /
  completion leakage). RAII handle ensures Completed/Failed on drop;
  failure mapping inherits parent R2.5 red 3 s hold. 159 trader-crate
  tests green; 34/34 anchors byte-identical (anchored bins never
  construct an EventBus). Open H3 (per-variant failure reason chip)
  deferred to v0.1.2. Commit: `c46fd45`. See
  `spec/cockpit-activity-llm-producer/feature.md`.

- **cockpit-training-pressed-wiring v0.1.0** — shipped 2026-05-27
  (operator-approved triple-batch). v0.1.1 follow-on of
  `cockpit-activity-status-bar v0.1.0` closing the Wave C T-D-N9
  ship-time open question — the Train button now actually trains.
  Binds `Message::TrainingPressed` in
  `crates/ui/src/bin/cockpit_live.rs::AppState::update` to call
  `lab::trainer::spawn_training_run` with default config
  `crates/forecast/train_tcn.toml` (Q1=(a)). New
  `crates/ui/src/lab/training_log.rs` (183 LoC) `TrainingLogRecipe`
  bridges std-mpsc training logs into the tokio runtime via
  spawn_blocking, surfacing per-epoch progress in the activity tape.
  Double-press inert per parent R3.4 (Q2=(a)). 34/34 anchors
  byte-identical; 5/5 integration tests · 0.31 s. K5 multi-toast
  follow-on opened as `cockpit-toast-queue v0.1.0` (Active). Commits:
  `28db398`, `c46fd45`. See
  `spec/cockpit-training-pressed-wiring/feature.md`.

### 2026-05-24 cohort

- **lab-yahoo-realdata v0.1.0** — shipped 2026-05-24 (operator-approved).
  Yahoo Finance pivot for the Lab UI: 10-ticker crypto-mirror universe
  (`BTCUSDT` … `LINKUSDT`), Binance-style symbols converted to Yahoo
  (`BTC-USD` …) at the dispatch boundary (Q6=(a) operator-override);
  adaptive cadence (1m ≤7d, 1h 7-60d, 1d >60d, Q4=(c)); parquet cache
  pattern + revision-pin mirroring the Binance precedent. New widgets:
  Source toggle (Synthetic / YahooCache), cadence badge. New crate path:
  `crates/data/src/yahoo.rs` + `fetch_yahoo_klines` CLI. New Venue::Yahoo.
  Anchor-additive contract preserved per ADR-0038 § D6.b — all 34
  anchors byte-identical (ScenarioConfig extensions use
  `#[serde(default, skip_serializing_if)]`). Tester Wave E PASS: 878+
  tests, T-C3.7 7/7 (yahoo-gated), clippy clean on touched crates,
  spec-lint baseline-stable. ADR-0040 codifies Yahoo realdata path +
  revision pin (`yahoo_finance_api = "=4.1.0"`). H1/H2 + cockpit-smoke
  + idle-CPU deferred to v0.1.1 per R6.3. Commits: `7ab924e`,
  `04e059f`, `a87bbc4`, `899c2a0`. See
  `spec/lab-yahoo-realdata/feature.md`.

### 2026-05-22 cohort

> 4 ships that day: 1 partial + 2 retire-with-evidence + 1 P0 wiring-fix.
> The Active blocks above (lines 376-742) carry full details; this section
> is the chronological pointer for future audits.

- **v3-llm-forecaster v0.1.0-PARTIAL** — shipped 2026-05-22 (operator-approved).
  First-of-kind `shipped-partial` precedent (code gates clean; Wave D deferred
  to v0.1.1 pending ANTHROPIC_API_KEY). 6 waves clean (A+B+C+E+F+G); 34/34
  anchors byte-identical; R9.3 byte-identity proven via SHA-256 match. ADR-0039
  LLM-forecaster verdict criteria L0-L4 codified. Wave D paused indefinitely
  per operator routing pick 2026-05-22. See `spec/v3-llm-forecaster/feature.md`.

- **v3-volatility-forecaster-noop-fix v0.1.0 (P0)** — shipped 2026-05-22.
  P0 wiring-bug fix: GARCH vol-target overlay was a no-op
  (`scale` computed but never applied to fill quantities). Discovery via
  orchestrator caveman probe (σ_hat × 2.95 → byte-identical equity → code review).
  Fix: `Strategy::quantity_scale` defaulted trait method; sizing hook at Buy arm;
  scale_cache + R2 forensic-gate test. ADR-0038 § D6.b anchor re-emission protocol
  amendment. 3 anchors re-emitted in-place (top10-2023-fy-vol-target-overlay-realdata
  + 2 sharpe-comparisons); vol-verdict-bs1-realdata stayed byte-identical.
  Post-fix equity: $113,479.98 → $62,807.89 (overlay actively destroys equity
  via GARCH-under-prediction × upper-clamp saturation; NEGATIVE-NET-DELTA confirmed).
  See `spec/v3-volatility-forecaster-noop-fix/feature.md`.

- **v3-volatility-forecaster v0.1.0** — shipped 2026-05-22; RETIRED same day.
  Hand-rolled GARCH(1,1) MLE + Parkinson estimator + V-verdict + 3 strategy
  builders + backtest scenario. Joint MODEL-BROKEN / NO-ALPHA / NEGATIVE-NET-DELTA
  verdict under real-wired overlay (post-noop-fix); operator routing R-O1 = (a)
  RETIRE C1. Code stays in tree; anchors locked. ADR-0038 V-verdict shape
  (now historical). See `spec/v3-volatility-forecaster/feature.md`.

- **v3-volatility-forecaster-rebaseline v0.1.0** — shipped 2026-05-22; RETIRED
  same day (with parent). Re-baseline pass per operator (b) routing pick
  from parent deck: new `top10-2023-fy-momentum-realdata` scenario + 1 anchor
  re-emitted. Architect locked NEW `ScenarioFamily::VolTargetRebaseline` to
  preserve parent anchor immutability (ADR-0038 § D6 contract held).
  Confirmed NO-ALPHA on real-vs-real comparison BEFORE the noop-fix discovery
  (the rebaseline verdict was correct conclusion, fortuitously — the noop-fix
  caveman probe later revealed the underlying bug). See
  `spec/v3-volatility-forecaster-rebaseline/feature.md`.

- **v2.5a — PatchTST forecast overlay (`v25a-patchtst-overlay` v0.1.0)** —
  shipped 2026-05-22 (operator-approved via presenter deck
  `presentations/v25a-patchtst-overlay-2026-05-22.md`;
  Q1-Q8 = analyst defaults via "Autoapprove all"; tester VERDICT →
  PASS after one-line K4 test-harness fix). Phase 2 of the
  4-phase DL roadmap.
  Predecessor: `v25-tcn-horizon-bump-or-retire v0.1.0`.
  Parent: `v25-dl-forecast-overlay v0.0.0` (now → terminal-retired per
  routing (a); see follow-on commit). **Substantive finding: F-verdict
  F4 with Sharpe-delta only +0.006144 vs v1 momentum baseline** — well
  below the +0.10 T-ALPHA-UNLOCKED threshold AND LOWER than retired
  v2.5 TCN (BS-1 @ 1h: +0.018; BS-2 @ 1h: +0.045). **Joint F4-F4-F4
  verdict across 3 model checkpoints / 2 model families (convolutional
  TCN + patch-attention PatchTST) / 2 horizons (1h + 24h)** establishes
  high-confidence retirement of the entire 4-phase DL forecast overlay
  roadmap (operator-decided routing (a) at presenter approval).
  Lands NEW `crates/forecast/src/patchtst.rs`
  (PatchTST model in candle; d_model=128, n_heads=4, n_layers=3,
  d_ff=256, patch_len=16, stride=8, context_len=336, dropout=0.2;
  ~431k params) + NEW
  `crates/forecast/src/bin/train_patchtst.rs`
  (training scaffold with ADR-0035 § D1 post-training σ_train pattern
  from the start — NOT the deprecated in-loop accumulator) + 4
  unit tests (forward_determinism / sigma_train_not_in_safetensors /
  tcn_byte_identity / patchtst_overlay_neutrality K4 anchor-
  neutrality test) + NEW `crates/strategy/src/patchtst_sync.rs` +
  NEW `crates/strategy/src/patchtst_overlay_momentum.rs` (sibling
  strategy mirror of `tcn_overlay_momentum.rs`) + NEW
  `crates/backtest/src/scenarios/patchtst_overlay_weights.rs`
  (sibling backtest scenario) + additive enum variants in
  `forecast_distribution.rs` + `sharpe_comparison.rs` + `backtest`
  Scenario enum. **2 new anchors locked under version
  `v2.5a.0-patchtst`** (30 total; 28 originals byte-identical):
  `forecast-distribution-patchtst-bs1-realdata` SHA `c55c6c51…` +
  `top10-2023-fy-patchtst-overlay-realdata` SHA `5f303cc0…`.
  Training stats: 30 epochs / 7h 45min wall-clock on Apple Silicon
  Metal / final train_loss 2.6e-5 (67× from epoch 1) / σ_train
  derived post-training 0.007053 (well-calibrated; in expected
  0.005-0.025 range). Checkpoint SHA `62520db9…` at
  `crates/forecast/checkpoints/anchors/patchtst-bs1-62520db9….{safetensors,metadata.json}`.
  ADR-0036
  codifies the PatchTST training contract.
  **Hypothesis status:** H1 (24h horizon unlocks signal where 1h
  failed) **FALSIFIED** — PatchTST @ 24h scored LOWER than TCN @ 1h
  on Sharpe-delta. H2 (attention captures session structure) =
  INCONCLUSIVE; F4 stays. H3 (4-6 week scope feasible) = CONFIRMED
  (actual <1 day end-to-end). H4 (σ_train post-training pattern
  works) = CONFIRMED. **Strategic implication**: v2.5-era DL
  approaches exhausted; pivot research budget per routing (a).
  Anchor risk zero — 28 originals + TCN checkpoint files byte-
  identical (verified via K4 neutrality test PASS on TCN scenario
  body SHA `8fa47f49…`); cargo fmt + workspace clippy +
  `--features candle` clippy + `--features candle,realdata`
  clippy + `--features forecast,forecast-audit-tick` clippy all
  clean; spec-lint 86/3 = baseline (0 new regressions); 2-run
  determinism PASS on all 3 substantive reports
  (forecast_distribution + backtest + sharpe_comparison).

- **v2.5 TCN horizon-bump or retire (`v25-tcn-horizon-bump-or-retire` v0.1.0)** —
  shipped 2026-05-21 as a **policy/decision feature** (no code change,
  no new anchors). Operator-decided Q1=(b) at the hard-blocker scope
  prompt: **retire v2.5 TCN at 1h horizon; pivot the multi-week budget
  to v2.5a PatchTST** (phase 2 of the 4-phase DL roadmap). Q2-Q7 MOOT
  under (b) — no retrain, no checkpoint, no new training anchor. The
  v2.5 TCN journey across 3 substantive ships
  (alpha-investigation v0.1.0 F4 verdict +
  recalibrate v0.1.0 σ_train bug eliminated +
  threshold-tuning v0.1.0 Joint T-MARGINAL
  +0.018 / +0.045) established that 1h-horizon TCN cannot extract alpha
  on real Binance OHLCV. **Decision rationale**: marginal +0.018 / +0.045
  Sharpe-delta is below the +0.10 alpha-unlock threshold AND a noise-floor
  question; ~4-6 weeks of PatchTST investigation is higher EV than ~2-3
  more weeks chasing a 24h-horizon TCN retrain when we already have
  evidence the model family struggles on hourly crypto bars. **What
  stays**: 28 existing anchors byte-identical (8 v2.5 TCN anchors +
  4 backtest-realdata + 4 v2.6.1-alpha-investigation-recalibrated +
  2 v2.6.2-threshold-tuning + 10 non-TCN); additive
  `with_tcn_bs{1,2}_ledger_tuned` builders shipped at threshold-tuning;
  ADR-0033 § D3 F-verdict + ADR-0035 σ_train recalibration contract
  remain in force as cross-phase invariants. **What promotes**:
  `v25a-patchtst-overlay` flagged ACTIVATION TRIGGERED in
  Queue § Strategy — promotes Queue → Active on next "next" directive.
  Trace row `REQ-V25-TCN-HORIZON-BUMP-OR-RETIRE-001` flipped
  draft → shipped (operator-decide as the load-bearing M-FINAL).

- **v2.5 TCN threshold tuning (`v25-tcn-threshold-tuning` v0.1.0)** —
  shipped 2026-05-21 (operator-approved via presenter deck
  `presentations/v25-tcn-threshold-tuning-2026-05-21.md`;
  Q1-Q6 = analyst defaults via "Autoapprove all"; tester VERDICT →
  PASS clean — all 9 T-F + 6 T-T gates green). Predecessor:
  `v25-tcn-recalibrate v0.1.0`.
  Parent (stays `in-progress`): `v25-tcn-overlay v2.5.0`. **Cheap τ × ε
  sweep follow-on** to the recalibrate ship — ran 90 backtests (9 τ ×
  5 ε × 2 checkpoints) over the recalibrated TCN checkpoints on real
  Binance OHLCV. **Substantive finding: Joint T-MARGINAL + T-MARGINAL**
  — headline cell on BOTH checkpoints is τ=0.1 / ε=0.001 with BS-1
  Sharpe-delta = **+0.018** and BS-2 = **+0.045**; both below the
  +0.10 T-ALPHA-UNLOCKED threshold. **No (τ, ε) tuple unlocks alpha.**
  F-verdict stays F4 per immutable
  ADR-0033 § D3;
  σ_train recalibration was necessary but not sufficient. Lands NEW
  `crates/backtest/src/bin/threshold_sweep.rs`
  (4-way `rayon::par_iter`, `(τ, ε)`-sorted assembly for byte-
  deterministic output) + NEW
  `crates/backtest/src/scenarios/threshold_sweep.rs`
  per-cell helper + **4 additive `_tuned` builders** on
  `tcn_overlay_momentum.rs`
  (`with_tcn_bs{1,2}_tuned` + `with_tcn_bs{1,2}_ledger_tuned`; explicit
  args required; `TcnSyncForecaster::with_direction_epsilon` builder +
  `direction_epsilon: Option<f32>` field with const-fold-default fallback
  so existing `_ledger` builders stay **literal `dec!(0.6)` + literal
  `forecast::tcn::DIRECTION_EPSILON`** — 26 predecessor anchors stay
  byte-identical, R-3 const-fold-default contract preserved). 2 new
  anchors locked under version `v2.6.2-threshold-tuning`:
  `threshold-sweep-bs1-realdata-recalibrated` (SHA `551cc2ab…`) +
  `threshold-sweep-bs2-realdata-recalibrated` (SHA `755bc380…`). T-
  classifier (T-ALPHA-UNLOCKED ≥+0.10 / T-MARGINAL [0, +0.10) /
  T-NO-ALPHA <0) embedded in report body per Q4=(c); ADR-0036 NOT
  written (deferred until empirical alpha-unlock evidence justifies
  codification). **Operator decided routing (c)** at presenter approval
  — ship advisory (additive `_tuned` builders + 2 new anchors) AND
  queue `v25-tcn-horizon-bump-or-retire` (currently
  ACTIVATION-TRIGGERED in Queue § Strategy; promotes on next "next"
  directive). H1 FALSIFIED (no tuple unlocked alpha); H2 confirmed
  (heatmap smoothness statistic in body); H3 confirmed (cheap sweep
  delivered clear verdict in hours). Anchor risk zero — 26 originals
  byte-identical; 28 total; cargo fmt + workspace clippy +
  `--features candle,realdata,forecast,forecast-audit-tick` clippy
  all clean; spec-lint 87/2 = baseline; 2-run determinism gate PASS;
  `git diff` over `crates/forecast/checkpoints/anchors/*.metadata*.json
  + *.safetensors` empty (ADR-0035 D4 invariant).

- **v2.5 TCN σ_train recalibration (`v25-tcn-recalibrate` v0.1.0)** —
  shipped 2026-05-21 (operator-approved via presenter deck
  `presentations/v25-tcn-recalibrate-2026-05-21.md`;
  Q1-Q5 = analyst defaults via "Autoapprove all"; tester VERDICT →
  PASS clean — all hard gates green). Predecessor:
  `v25-tcn-alpha-investigation v0.1.0`.
  Parent (stays `in-progress`): `v25-tcn-overlay v2.5.0`. Metadata-
  only fix to the σ_train scalar in the BS-1 + BS-2 TCN anchored
  checkpoints — the predecessor's F-verdict investigation surfaced a
  **608× / 580× σ_train inflation** caused by an in-loop accumulator
  pattern at `train_tcn.rs:606,676-678,733-741`
  that never reset `all_r_hats` between epochs, so the final scalar
  was dominated by pre-convergence trajectory variance instead of
  the converged-model prediction std. Lands a NEW
  `crates/forecast/src/bin/recalibrate_sigma_train.rs`
  (~490 LoC, `--features candle`-gated) + additive `--metadata-path`
  flag on `forecast_distribution.rs` (default behaviour byte-identical
  → 22 anchor SHAs preserved) + 3 new unit tests
  (`recalibrate_sigma_train_readonly`,
  `recalibrate_sigma_train_field_invariance`,
  `sigma_train_not_in_safetensors`). New anchors locked under version
  `v2.6.1-alpha-investigation-recalibrated`: 4 total — 2 forecast-
  distribution recalibrated bodies + 2 derivation reports. Original
  `.metadata.json` + `.safetensors` files **byte-identical**
  (verified: `git diff HEAD -- crates/forecast/checkpoints/anchors/*.metadata.json`
  empty). ADR-0035
  codifies the cross-phase σ_train recalibration contract (overlay
  file convention + on-disk JSON number divergence from ADR-0029 §2
  rule 5 + σ_train-not-in-safetensors invariant) so the same bug
  shape can't reappear in v2.5a PatchTST / v2.5b Transformer
  training scaffolds. **Substantive findings:**
  (i) σ_train bug confirmed real, eliminated (BS-1: 10.954 → 0.018;
  BS-2: 6.916 → 0.012). Both recalibrated values in expected
  0.005..0.025 range. (ii) **F-verdict stays F4** per immutable
  ADR-0033 § D3
  priority tree (`frac_inside_epsilon` 0.031 / 0.057 < 0.5 F3
  threshold). (iii) **BUT gate-survival jumps dramatically**:
  BS-1 τ=0.6: **0% → 40.1%**; BS-1 τ=0.1: **0% → 88.8%**; BS-2
  similar magnitude. Surfaced standalone per Q4=(c) as the
  `## Recalibration delta` section in each recalibrated report.
  σ_train is no longer a confounding variable in the v2.5 TCN model
  assessment. **Routing decided 2026-05-21 — option (c)**: queue
  both `v25-tcn-threshold-tuning` (cheap τ-sweep, hours-not-weeks)
  and `v25-tcn-horizon-bump-or-retire` (multi-week retrain or
  retire v2.5 TCN for v2.5a PatchTST); threshold-tuning ships
  first; horizon-bump-or-retire as fallback if τ-sweep finds no
  alpha. **Anchor risk zero** — 22 originals byte-identical;
  cargo fmt + workspace clippy + `--features candle` clippy all
  clean; 7 new integration tests PASS; spec-lint 87/2 = baseline
  (0 new categories). Trace row `REQ-V25-TCN-RECALIBRATE-001`
  flipped `draft → shipped`.

- **UI rethink Phase F — Memory + Models + Phase-6 Assistant slot
  (`ui-rethink-phase-f-memory-models-assistant` v0.1.0)** —
  shipped 2026-05-21 (operator-approved via "Autoapprove all" against
  presenter deck
  `presentations/ui-rethink-phase-f-memory-models-assistant-2026-05-21.md`;
  Q1-Q8 = analyst defaults; tester VERDICT → PASS clean — sole
  deferral is H3 idle-CPU 60-s probe in the display-server class).
  Predecessor:
  `ui-rethink-phase-e-compare v0.1.0`.
  **SIXTH AND FINAL PHASE OF THE UI RETHINK** — closes the
  `docs/dev-notes/ui-rethink-2026-05-17.md`
  redesign per §6 line 1134 ("No cliffs at C, E, F — each phase is
  independently shippable and independently reversible"). Lands all
  three deferred surfaces from dev-note §6 Phase F (lines 1098-1112):
  (i) **Memory screen** (J7) over `crates/reflection` `lesson_cards`
  store via NEW `crates/reflection/src/query.rs`
  (`list_recent_lesson_cards` / `open_and_list_recent`; UI receives
  via `Message::MemoryHydrate` — Phase D `trail_mirror` precedent
  per K1 architect resolution); reverse-chrono list + side drawer
  for entry detail; Memory→Trail chevron back-link via existing
  `OpenTrailFor` compound dispatch (additive, no Phase D body touch).
  (ii) **Models screen** (J8) over
  `crates/forecast/checkpoints/anchors/` — BS-1 + BS-2 TCN
  checkpoints inventoried via hand-parsed JSON (`#[serde(default)]`
  on every non-load-bearing field; 5 H5 unit tests cover schema
  drift); flat list with all entries rendered as "staged" at v0.1.0
  (Q7=(c)). Sparkline DEFERRED to v0.2.0 (replay-cache forecast
  namespace empty per K3 — `—` placeholder + tooltip).
  (iii) **Phase-6 Lumen Assistant slot** wakes structurally — NEW
  additive `RIGHT_RAIL_OPEN_WIDTH_PX = 320.0` (`theme.rs:~644`); old
  `RIGHT_RAIL_WIDTH_PX = 0.0` constant **preserved verbatim** per K6
  Option A so Phase D `trail_drawer.rs` stays byte-identical (R7.2 —
  T-F10 `git diff` confirmed 0 lines). Q4=(a) stub-only content
  ("Assistant offline. v2 LLM wiring lands in v0.2.0."). **K4
  resolution**: Memory drawer (centre body) + Assistant slot (far-
  right shell track) live in DIFFERENT shell columns — no right-side
  conflict. **12 net-new source files** + **6 new snapshot baselines**
  (`memory__cold_boot_empty`, `memory__steady_state_5_cards`,
  `memory__drawer_open_on_card_click`,
  `models__cold_boot_no_checkpoints`,
  `models__steady_state_2_checkpoints`,
  `assistant_slot__open_stub`); zero new external crate deps; zero
  new architecture edges. **311 lib tests PASS** (309 → +2 from
  Phase E); **ANCHORS PASS (22/22)** pre- AND post-sweep;
  layout_invariants 10/10 (7 carry-forward + 3 new = 768 panic-free
  proptest cases for the new screens); shell_grid 3/3
  (`RIGHT_RAIL_WIDTH_PX = 0.0` invariant preserved); 6 snapshot tests
  deterministic on rerun; fmt + clippy clean (default AND
  `--features live`); spec-lint 87 (= Phase E baseline; 0 new).
  H1 (cold-boot read latency) NOT FALSIFIED (reflection.db absent →
  0-row sub-ms path; static argument under load); H2 (checkpoint
  parse) NOT FALSIFIED (855B + 852B JSON ≈ 20 μs, ~50000× headroom
  over 50ms p99 budget); H4/H5/H6 all PASS; H3 idle-CPU deferred
  (display-server class). **v0.2.0 / Phase G candidates surfaced**:
  Memory cluster mode (reflection-memory distillation); Memory
  sparkline (replay-cache forecast namespace population);
  Q4=(b) full v2 LLM text-stream wire for Assistant slot; J5
  writer-side affordances; serving-status pill lifecycle.

- **UI rethink Phase E — Compare matrix (`ui-rethink-phase-e-compare` v0.1.0)** —
  shipped 2026-05-20 (operator-approved via "Autoapprove all" against
  presenter deck
  `presentations/ui-rethink-phase-e-compare-2026-05-20.md`;
  Q1-Q8 = analyst defaults; tester VERDICT → PASS clean, **no v0.1.0
  deferrals**). Predecessor:
  `ui-rethink-phase-d-trail-followup v0.1.1`.
  Fifth concrete feature in the UI rethink at
  `docs/dev-notes/ui-rethink-2026-05-17.md`.
  Lands the **read-only Compare matrix** (J3) — 6 strategies × ≤10 pairs
  grid that reads cached report frontmatter under `spec/<strategy>/reports/`
  via the new `crates/ui/src/compare/cache.rs` hand-parser (no
  `serde_yaml` dep — K3 architect resolution). Cell click →
  `Message::OpenLabFromCompare { strategy, pair, range }` compound
  dispatch (mirrors Phase D `OpenTrailFor`). Empty cells render a
  per-cell **Run** affordance routed through the Phase B Lab Run
  round-trip (Q4=(b)). Greyed cells for tuples outside a strategy's
  declared universe (Q8=(b)). Universe-aggregate KPI cells (Q6=(a))
  carry a **dual-surface disclaimer** (subtitle + per-cell tooltip)
  per the architect's K7 mitigation upgrade. Sidebar entry already
  reserved by Phase C — only the body route swaps from
  `placeholder::view` to `screens::compare::view` at
  `crates/ui/src/shell.rs:96`. **5 net-new files** (`compare/mod.rs`,
  `compare/state.rs`, `compare/cache.rs`, `widgets/matrix.rs`,
  `screens/compare.rs`). **Zero new external crate deps; zero new
  architecture edges; zero anchor risk by construction.** **946 lib
  tests PASS** (939 baseline → +7 new: 5 cache + 2 H5 round-trip);
  **ANCHORS PASS (22/22)** pre- AND post-sweep; layout_invariants
  7/7 (6 carry-forward + new `compare_screen_no_zero_dim` 256-case
  proptest); 4 new snapshot baselines (`compare__cold_boot_all_empty`,
  `compare__steady_state_populated`,
  `compare__empty_cell_run_affordance`,
  `compare__column_header_hover` — byte-identical to
  cold-boot-all-empty by R2.4 design since v0.1.0 column headers are
  non-interactive); fmt + clippy clean (both default AND
  `--features live`); spec-lint 87 (= predecessor baseline, 0 new).
  H1 = **40 % first-open cache hit rate** (24/60 cells per architect
  static census; ≥30 % threshold); H4 = **≤15 ms p99 by static
  argument** (shell-level glob+head over 32 reports at 0.12 s
  wall; Rust ≥10× faster). **v0.2.0 / Phase E.1 candidates**:
  per-pair backtest decomposition (true per-pair Sharpe, closes Q6
  with (c) fallback); background recompute orchestration (Q2 (a)/(b));
  in-session cache invalidation.

- **UI rethink Phase D+ — Trail follow-up (`ui-rethink-phase-d-trail-followup` v0.1.1)** —
  shipped 2026-05-20 (operator-approved via "Autoapprove all" against
  presenter deck
  `presentations/ui-rethink-phase-d-trail-followup-2026-05-20.md`).
  Predecessor: `ui-rethink-phase-d-trail v0.1.0`.
  Closes T-D-N26 (iced **Subscription bridge** wiring
  `reflection::trail_mirror::TrailMirrorTick` into `Cockpit::subscription`;
  Q3=(c) — handle constructed in `cockpit_live.rs` bootstrap + stored on
  `AppState`), T-D-N27 (**3 new insta snapshot baselines** —
  `trail__steady_state`, `trail__side_drawer_open`,
  `live__recent_activity_with_chevron`; NEW baselines, not changes
  to anchored body-SHAs), and T-D-N29 (**H5 backfill-latency bench**
  `crates/reflection/benches/trail_mirror.rs` — p99 = **0.021 ms** ≪
  50 ms; H5 NOT falsified, ~2380× headroom). UI-local wrapper types
  (`TrailMirrorUiTick` / `TrailStageUi` / `ReconstructedTrailUi`) at
  `crates/ui/src/state.rs:~1340` keep `ui`'s default-build edge graph
  free of `reflection` (Q2=(b)); `reflection` joins as `optional = true`
  behind the existing `live` feature stanza — **zero new architecture
  edges** in the data-flow sense, ADR-0031 carry-forward honored.
  Idle-CPU sampler `scripts/bench_idle_cpu.sh` (Q4=(a) macOS `top`).
  **Deferred to v0.1.2** (sandbox display-server class, same as
  predecessor): T-F6 idle-CPU 60-s sustained probe + T-F7 K7 paper-
  mode ForecastEmitted counter (Q1=YES — deployment-side run by
  operator) + `--features live` clippy hygiene (13 pre-existing
  `needless_pass_by_value` in `crates/ui/src/live.rs:159-428`).
  **939 lib tests PASS** (≥ 937 baseline; +2 new state-tests);
  **ANCHORS PASS (22/22)** pre- AND post-sweep; layout_invariants
  6/6 PASS; 3 snapshot tests deterministic-on-rerun; fmt + default
  clippy clean; spec-lint 87 (0 regression vs predecessor baseline).

- **UI rethink Phase D — Trail view (J4) (`ui-rethink-phase-d-trail` v0.1.0)** —
  shipped 2026-05-20 (operator-approved via "Autoapprove all" against
  presenter deck
  `presentations/ui-rethink-phase-d-trail-2026-05-20.md`;
  five deferred items — T-D-N26 Iced Subscription bridge, T-D-N27 3
  snapshot baselines, T-D-N29 H5 backfill-latency bench, T-F6 idle-CPU
  floor, T-F7 K7 paper-mode counter — explicitly accepted as Phase D+
  v0.1.1 follow-up scope; wiring confirmed by inspection).
  Predecessor: `ui-rethink-phase-c-sidebar-ia v0.1.0`.
  Fourth concrete feature in the UI rethink at
  `docs/dev-notes/ui-rethink-2026-05-17.md`.
  Lands the **decision-trail visualisation** of the multi-agent pipeline
  — Fill → Signal → Forecast as a stacked node graph via new
  `widgets::trail_node` + `widgets::trail_drawer` + `screens::trail`.
  Universal Trail chevron in Live recent-activity + audit table rows.
  **First downstream consumer of `audit-tick-consumer-envelope v0.1.0`**
  — closes T-D-14 via `TcnForecaster::with_ledger` runtime wiring at
  `crates/strategy/src/tcn_overlay_momentum.rs:417-420,434-437` +
  `post_forecast_event` emits at `crates/forecast/src/tcn.rs:861-879,997-1010`.
  **Mig 011 (anchor-safe additive)** — 4 ALTERs (NULL-default) + new
  `forecast_events` table + 4 indexes — `ANCHORS PASS (22/22)` post-mig
  (H2 confirmed). 937 lib + integration tests PASS; trail-reconstruction
  3/3 PASS; M1-C layout invariants 6/6 PASS (CI-safe cockpit-smoke
  proxy); fmt + clippy clean; spec-lint Phase D contribution = 0 new
  categories (91 violations / 2 categories vs 734/3 baseline). ADR
  amendment at
  `adr/0031-audit-tick-consumer-envelope.md`
  § "Phase D amendment (2026-05-20)".

- **UI rethink Phase C — Sidebar IA flip + Live + Strategy registry + Settings rollup (`ui-rethink-phase-c-sidebar-ia` v0.1.0)** —
  shipped 2026-05-20 (operator-approved via "Autoapprove all" against
  presenter deck
  `presentations/ui-rethink-phase-c-sidebar-ia-2026-05-20.md`;
  K1/K2 gut-check questions accepted as not-blockers — revisitable in
  Phase D). Predecessor:
  `ui-rethink-phase-b-lab-run v0.2.0`.
  Third concrete feature in the UI rethink at
  `docs/dev-notes/ui-rethink-2026-05-17.md`.
  Lands the **three-group sidebar IA** (Work zone Lab · Live ·
  Compare; Library zone Strategies · Memory · Models · Trail; Chrome
  zone Settings) with hairline `BORDER_1` dividers — entries
  unchanged from `SIDEBAR_ENTRIES_PHASE_A`, only their visual
  relationship changed. **`Live` screen** replaces the deprecated
  `Home` 2×2 grid with the dev-note §J6 layout (system-health strip
  + equity curve + KPI strip + positions + activity + placeholder
  LLM tile). **`Strategy registry`** replaces the panel-style
  `strategies::view` with a list-of-cards layout (status pill +
  universe + last-anchor + last-live-run + "Open in Lab" action).
  **`Settings` rollup** revives the dead-code `risk::view` /
  `control::view` / `debug::view` bodies under a three-tab wrapper
  (Risk · Control · Debug, default tab = Risk). One-cycle compat
  shim for deprecated `Screen::*` variants — Phase D prunes per Q1a.
  **5 net-new files** (3 screens + 2 widgets); **1 new public
  Message variant** (`SwitchSettingsTab(SettingsTab)`); no ADR
  (UI-layout scope). **22 body-SHA anchors byte-identical** (zero
  anchor risk by construction); 287 lib + 101 integration tests
  PASS; 6 new snapshot baselines + 5 refreshed; cockpit-smoke 0
  panics; spec-lint Phase C contribution = 0. **Real-world
  confirmation:** operator exercised the live cockpit this session
  and confirmed chart + hovering work end-to-end (post chart-fixture-
  line-clipping v1.0.0).

- **Audit tick consumer envelope (`audit-tick-consumer-envelope` v0.1.0)** —
  shipped 2026-05-20 (operator-approved via "Autoapprove all" against
  presenter deck
  `presentations/audit-tick-consumer-envelope-2026-05-20.md`;
  open Q on T-D-14 deferred to Phase D per presenter's recommendation).
  Predecessor: ADR-0031 (status `proposed → accepted` at architect
  M-T1). Adds a thin read-direction envelope (`AuditTick<E, C>`) over
  the existing audit journal: 8 in-scope `journal::*` writers enqueue
  `AuditTick`s into a `tokio::broadcast` channel; `crates/reflection`
  carries an observation-only stub consumer (gated by
  `[reflection].audit_tick_consumer_enabled = false` — keeps default
  behaviour bit-identical). **Opt-in by construction:** `Ledger::open`
  produces no tee; only the new `Ledger::open_with_tick_bus`
  constructor wires the channel. **22 body-SHA-256 anchors
  byte-identical**; spec-lint feature contribution = 0; 6 new test
  files + 1 bench file under `crates/audit/`; ForecastEmitted call
  site pinned at `crates/forecast/src/tcn.rs:786-795` (cache-hit) +
  `:889-898` (post-inference), feature-gated `audit-tick`. **Deferred
  runtime wiring** — T-D-14 (`strategy` crate optional `Ledger`
  handle) waits until Lab Trail (Phase D) needs ForecastEmitted at
  runtime; no current consumer reads it, so closing earlier would
  land dead code. ADR-0031 + `01-data-flow.md` updated.

- **Chart fixture line clipping (`chart-fixture-line-clipping` v1.0.0)** —
  shipped 2026-05-20 (operator-directed overnight fix). **Root cause:**
  iced 0.14.0 `tiny_skia` backend has a transformation-order bug in
  `Renderer::draw_primitives` (canvas group primitives applied with
  `group.transformation() * scale_factor` instead of
  `scale_factor * group.transformation()`, plus duplicate clip_bounds
  multiplier). The bug clips canvas geometry to a bottom-right sub-region
  of the canvas widget bounds. **Fix:** backport iced master commit
  `76b32d4906`
  (Jan 28, 2026) via `vendor/iced_tiny_skia/` + workspace
  `[patch.crates-io]`. **Operator-locked 2026-05-20:** the vendored
  fork is the long-term canonical fix (no iced 0.14.x patch branch
  exists; no upgrade expected near-term). Any future iced bump audits
  the `Transformation::scale(scale_factor) * group.transformation()`
  ordering before retiring the fork. **Verification:** 4 visual_snapshots
  baselines refreshed (chart line now spans full 12:00→12:59 width);
  22/22 anchors byte-identical; 279 workspace tests PASS; cockpit-smoke
  0 panics. Diagnostic trail in
  `spec/chart-fixture-line-clipping/feature.md`
  preserves the orchestrator's 5-hypothesis probe register + 2 falsified
  fix attempts + final root-cause analysis.

- **Chart x-axis local time (`chart-x-axis-local-time` v1.11.0)** —
  shipped 2026-05-20 (operator-approved via "Autoapprove all"
  against presenter deck
  `presentations/chart-x-axis-local-time-2026-05-20.md`).
  Predecessor: `chart-canvas-overhaul v1.10.0`.
  Closes the operator-friendly local-time landing deferred from
  v1.10.0 by Q-revised-1 = path (b). Trivial direct ship per
  CLAUDE.md (no analyst/architect sub-agent cycle): 1-line
  `Cargo.toml` edit adding `"local-offset"` to the `time` crate's
  features array; ~10 LOC in `crates/ui/src/widgets/chart.rs`
  splitting `local_offset_or_utc()` into a `#[cfg(test)]` UTC branch
  + a `#[cfg(not(test))]` production branch that reads
  `time::UtcOffset::current_local_offset()` with defensive
  `unwrap_or(UtcOffset::UTC)` fallback; 1 new unit test pinning the
  `cfg(test)` UTC contract. **Snapshot determinism preserved across
  host time zones** via a complementary `UI_CHART_FORCE_UTC` env-var
  gate set at the top of both integration test runners
  (`tests/render_snapshots.rs:run_panel_slot` +
  `tests/visual_snapshots.rs:run_slot`) — this corrects a latent
  issue in the predecessor M7 architect's "cfg(test) override
  holds" claim (Cargo only sets `cfg(test)` on a crate when building
  it as a test target; integration tests link against the library
  compiled WITHOUT `cfg(test)`, so the unit-test branch alone is
  insufficient). **22 / 22 anchors byte-identical** (R10.1; no
  strategy / audit / exec / report path touched); 279 workspace
  tests PASS (+1 vs Phase B baseline); cockpit-smoke PASS 0 panics;
  fmt + clippy clean; spec-lint Phase contribution = 0.

- **UI rethink Phase B — Lab Run button (`ui-rethink-phase-b-lab-run` v0.2.0)** —
  shipped 2026-05-19 (operator-approved via "Autoapprove all" against
  presenter deck
  `presentations/ui-rethink-phase-b-lab-run-2026-05-19.md`;
  6 manual `[orchestrator]` acceptance rows — H1 latency p95, H5 idle-CPU
  floor, H7 mirror RSS delta, K3 cancel-on-shutdown live test, Δ-KPI badge
  visual capture, Phase C bar-level cancel-poll scope — auto-cleared by
  the same blanket approval). Predecessor:
  `ui-rethink-phase-a-lab v0.2.0`.
  Second concrete feature in the broader UI rethink at
  `docs/dev-notes/ui-rethink-2026-05-17.md`.
  Promotes Phase A's stubbed Lab `Run` button to a real in-process
  backtest call closing the operator's J2 workflow end-to-end.
  **Headline:** `crates/backtest/src/main.rs` collapsed **3417 → 1447
  LOC** (-57%); scenario bodies extracted into
  `crates/backtest/src/scenarios/{momentum,pairs,sma_composed,tcn_overlay,tcn_overlay_weights}.rs`
  and report writers into `crates/backtest/src/report/*`;
  `engine::run_scenario` dispatches via mapping layer
  (`ScenarioConfig` → per-scenario input → unified `RunReport`); new
  `LabState.last_run_report`/`prev_run_report` rotation + new
  `widgets::run_delta_badge` (Δ P&L / Δ MaxDD / Δ Sharpe). **22/22
  anchors byte-identical** (extraction is behaviour-preserving by H2/H4
  construction); cockpit-smoke 0 panics; spec-lint Phase B contribution
  = 0; 278 workspace tests + 10 new engine::tests PASS; 5 operator-
  decide Qs all resolved to analyst-recommended defaults (Q1=A in-memory
  return; Q2=A `ThrottledSpinner` only; Q3=A disabled-while-running +
  internal cancel poll; Q4=A session-local diff; Q5=A preserve all 22
  anchors). **Known deviation (Phase C deferred):** cancel uses wrap-
  and-abort (`tokio::spawn` + drop on cancel) instead of ADR-0035 D6's
  bar-level `bar_idx & 0x7F == 0` polling; bar-level threading deferred
  to a Phase C work item. ADR-0035 (scenario-dispatch extraction pattern)
  landed. See
  `spec/ui-rethink-phase-b-lab-run/feature.md`.

- **Cockpit performance + input responsiveness (`cockpit-performance-and-input-responsiveness` v1.0.0)** —
  shipped 2026-05-15 (operator-approved via presenter deck
  `presentations/cockpit-performance-and-input-responsiveness-2026-05-15.md`;
  this backlog entry was stale until 2026-05-19 spec-hygiene sweep).
  Predecessor: `ui-quality-gate-overhaul v1.0.0`. **Headline: idle CPU
  dropped from ~66.9% → 2.2-13.1%** on the fixtures-mode cockpit
  (~18× typical / 30× peak). M0 samply 0.13.1 profile identified the
  dominant hot path as `iced_tiny_skia::Compositor::present` at 45.5%
  inclusive + `draw_quad` at 20.5% + tiny-skia pixel pipeline at 27%+
  — i.e. continuous full-frame software-rasterized repaints at idle.
  H-PERF-1 CONFIRMED-INDIRECT, H-PERF-2 + H-PERF-4 CONFIRMED, H-PERF-3
  deferred. **M1 fix (shipped):** new `crates/ui/src/widgets/throttled_spinner.rs`
  wraps `iced_aw::Spinner` and gates its `RedrawRequested` subscription
  from **60 fps → 10 fps** (the spinner still animates smoothly; the
  cockpit's CPU stops melting). **M1B (Table memoization) + M1C
  (hit-test) NOT shipped** — post-fix CPU was already in single-digit
  range so they remain queued in tasks.md as conditional sub-targets
  for any future regression. Evaluator PASS 15/15; 280 default-feature
  tests + 286 under `--features render-debug` = 280/286 PASS.

- **Cockpit training control (`cockpit-training-control` v0.2.0)** —
  shipped 2026-05-19 (operator-approved via "Autoapprove all" against
  presenter deck
  `presentations/cockpit-training-control-2026-05-19.md`;
  3 manual `[orchestrator]` acceptance rows auto-cleared by the same
  blanket approval). Predecessor:
  `ui-rethink-phase-a-lab` v0.2.0.
  Integrates `train_tcn` model training into the cockpit UI as the
  natural workflow surface for the upcoming v2.5 retraining cycle and
  v2.5a/v2.5b future training rounds. Two-tier scope landed:
  **Tier 1** = Lab Train sub-panel (collapsible, bottom of Lab column)
  + subprocess spawn via `lab::trainer::spawn_training_run` (mirrors
  `lab::runner` cancellation-handle pattern) + 200-line ring-buffer
  `training_log` widget + SIGKILL-immediate Cancel semantics.
  **Tier 2** = additive SQLite migration 010 introducing the
  `training_events` table + opt-in `--audit-db <PATH>` flag on
  `train_tcn` (default omitted; byte-identical CI runs preserved) +
  1-Hz audit-DB poller iced Subscription recipe + `widgets::training_plot`
  loss-curve plot + `widgets::axis` shared Lumen primitive + cross-platform
  `pid_alive` helper + status-strip orphan-detect annotation.
  **Non-regression contract (R10) honored:** 22/22 anchors byte-identical
  (zero new anchors locked — training inputs include wall-clock + UUID
  surfaces that preclude byte-identity); cockpit-smoke PASS (0 panics in
  8s window); cockpit-training-control's own spec-lint contribution = 0;
  9 new snapshot tests + 3 new tests for `pid_alive` + 3 for
  `training_subscription` + 3 for `widgets::training_plot` + 4 for
  `training_status_strip` + 6 for `widgets::axis` + golden-CLI gate (K5
  mitigation). T-D-N1..T-D-N18 (all 18 dev rows) ticked at commit `6e5b884`;
  orchestrator-only render-baseline refresh at commits `8d1edf4`+`5ce42e6`
  (legitimate composition drift from Train sub-panel addition).
  See `spec/cockpit-training-control/feature.md`.

- **Real-Binance-data backtest path (`backtest-real-binance-data` v0.1.0)** —
  shipped 2026-05-18 (operator-approved via presenter deck
  `presentations/backtest-real-binance-data-2026-05-18.md`).
  Predecessor: `v25-tcn-overlay` v2.5.0 M3.
  Wires the backtest harness to read real Binance hourly parquet from
  `data/binance/` via a new `realdata` cargo feature (opt-in; default
  build never compiles the new module). New `data::revision` module
  emits + verifies a `REVISION.toml` per-file SHA-256 manifest. Four
  new `-realdata` scenarios, four new anchors under version
  `v2.6.0-realdata` (`top10-{2023,2024}-fy-tcn-overlay[-weights]-realdata`).
  19/19 anchors total; 15 originals byte-identical. **Open finding:**
  TCN real-weights produces `dampened=0` on real Binance OHLCV too —
  not a regression but unblocks the next investigation (`v25-tcn-alpha-investigation`,
  queued above). See `spec/backtest-real-binance-data/feature.md`.

- **UI rethink Phase A — chart-centric Lab (`ui-rethink-phase-a-lab` v0.2.0)** —
  shipped 2026-05-18 (operator-approved via presenter deck
  `presentations/ui-rethink-phase-a-lab-2026-05-18.md`).
  Predecessor: `chart-canvas-overhaul` v1.10.0.
  Renames `Charts → Lab`, flips Lab to the default boot route, fuses three
  overlay layers on the single canvas (buy/sell markers + equity curve +
  ≤4-strategy comparison), adds pair-chip / strategy-chip / date-range
  widgets, persists `(strategy, pair, range, params)` with cold-start
  defaults `v1.momentum × XRPUSDT × Last 90d` (Q-A3). 358/358 ui tests +
  20/20 determinism + 13/13 anchors. Visual A/B captures deferred to
  operator-local. See
  `spec/ui-rethink-phase-a-lab/feature.md`.

- **Drop iced_aw + iced_fonts (`ui-drop-iced-aw` v0.1.0)** — shipped
  2026-05-16. Strategic decoupling from third-party iced ecosystem
  cadence after the 2026-05-16 aborted comet bump made the
  ecosystem-lag pattern explicit. spinner already self-replaced by
  `widgets/throttled_spinner`;
  badge replaced with native Container+Text in
  `widgets/strategies::status_badge_cell`
  using the same Lumen palette pairs; date_picker (smoke-test demo
  per docstring) removed entirely with its state, messages, and
  snapshot test. `cargo tree -p ui` confirms zero iced_aw +
  iced_fonts. 1216 workspace tests pass (-8 deleted-as-expected),
  anchors 11/11 PASS. **Net effort: ~3h actual vs ~18h estimate** —
  the date_picker docstring saved 2 dev-days of mistaken
  reimplementation. See
  `spec/ui-drop-iced-aw/feature.md`.

- **headless emulator adapter (`ui-headless-emulator` v0.1.0)** —
  shipped 2026-05-16. Decomposed out of `ui-test-harness-ci` to
  close the unchecked "headless mode" cell from
  `iced-014-feature-analysis-2026-05-15.md §4`
  without waiting on viewport-matrix + evaluator prereqs. Single
  test (`crates/ui/tests/headless_emulator_smoke.rs`) boots the
  cockpit through `iced_test::emulator::Emulator`, drains events
  until `Ready`, takes a 1280×720 screenshot — proves the FULL
  iced subscription pump runs without a window server. 1224
  workspace tests pass (+1). ~1 hour actual vs ~2.25h estimate.
  See `spec/ui-headless-emulator/feature.md`.

- **session journal — iced_tester adapter
  (`ui-session-journal-iced-tester` v0.1.0)** — shipped 2026-05-16
  (commit `218cab3`). Adapter for iced 0.14's `iced_tester::attach`
  (recorder overlay) + `iced_test::run` (replay). Built with
  `--features record-tests` auto-attaches overlay; production
  builds untouched. Empty `recorded-sessions/` ships; operator
  populates post-ship via the recorder workflow. See
  `spec/ui-session-journal-iced-tester/feature.md`.

- **iced native widgets (v0.1.0)** — shipped 2026-05-13
  (operator approval recorded as `[x] Approved — ship` in
  `spec/iced-native-widgets/presentations/iced-native-widgets-2026-05-13.md ## Approval`;
  evaluator `VERDICT → PASS` at
  `reports/evaluation-2026-05-13T10-45Z.md`
  on commit `1431409`). Brief A of the iced ecosystem evaluation
  (predecessor v0.2.0) — 4
  hand-rolled cockpit widgets migrated to iced 0.14 native widgets:
  - `crates/ui/src/widgets/positions.rs` → native `Table`
    (commit `9027a0d`, M1 / R1)
  - `crates/ui/src/widgets/strategies.rs` → native `Table` with
    Button-in-column-1 row-click + sibling `Column<error_badges>`
    (commit `3077425`, M2 / R2)
  - `crates/ui/src/widgets/kpi_strip.rs` → native `Grid::new()
    .columns(6).spacing(space::M).height(Length::Shrink)`
    (commit `970e857`, M3 / R3)
  - `crates/ui/src/widgets/journal_transaction_modal.rs` → native
    `Float` positioning wrapping a 3-layer `Stack` (commit
    `9e5bd65`, M4 / R4)
  New shared theme submodule: `crates/ui/src/theme/iced_widget_catalogs.rs`
  exposes `cockpit_table_style_fn` factory (commit `3077425`, T2.0)
  for Brief B `iced_aw` adoption to consume. Native v0.14 `Table::new`
  has no `.style()` setter, so the factory is unused in v0.1.0 —
  consumption deferred to Brief B / v0.2.
  **4-lane parallel dev fan-out worked**: Lanes 2/3/4 spawned in
  parallel (different files, zero overlap); Lane 1 sequenced after
  Lane 2's T2.0 Catalog adapter committed. Each lane = one
  per-widget commit (4 dev commits + 1 tester commit `1431409`).
  Workflow firsts proven:
  - **Second invocation of the test-runner / evaluator split** (first
    was `ui-test-harness-bootstrap` v0.1). Evaluator default-FAIL
    contract held; 20/20 V-items (V1A-V4E) PASS in fresh context.
  - **Orchestrator-direct M0 falsifier batch** (T-M0-J through
    T-M0-N) — 5 grep checks the sub-agent sandbox couldn't run,
    completed in one orchestrator shell pass before dev fan-out
    spawned. Caught 2 architect-spec corrections (`Float::new(1
    arg)` not 2; orphan-rule violation on `impl Catalog`) before
    code was written.
  - **`scripts/orch_supplement_log.sh`** (tooling extracted from
    bootstrap v0.1) supplemented 3 sandbox-denied checks
    (`cargo doc`, shasum, clocks-grep) into the test-runner's
    log — pattern repeatable.
  4 honest architectural divergences flagged inline (orphan-rule
  pivot to StyleFn factory; `Grid::height(Shrink)` AspectRatio
  override; `Float::new(1 arg)` not 2; `Table::new` accepts
  `IntoIterator<Item=T>` looser than `Vec<T>`).
  **Net LOC** +154 (+47 positions / −30 strategies / +8 kpi /
  +29 journal / +100 new Catalog adapter) — the predecessor brief's
  "−900-1100 LOC retired" framing measured file span, not glue
  layer; **actual value is standardization** (idiomatic iced
  widgets, future-proof AccessKit hooks, less hand-rolled
  responsibility, theme adapter scaffold for Brief B). Anchor
  neutrality preserved: 11/11 byte-identical; bootstrap V8 visual
  baseline check carry-forward — 3 PNG SHAs byte-identical
  (Charts screen unaffected). 1203+ workspace tests passing.

- **v2 LLM strategy (v2.0.0)** — shipped 2026-05-13
  (operator approval recorded as `[x] Approved — ship` in
  `spec/v2-llm-strategy/presentations/v2-llm-strategy-2026-05-13.md ## Approval`;
  tester `VERDICT → PASS` at
  `reports/test-2026-05-12-2219-v2-llm-strategy-final.md`
  on commit `8a41b47`). Foundation-only per **Q1=A** —
  `Strategy` trait unchanged; first consumer briefs queued
  (reflection-memory-llm-enrichment + reflection-memory-trader-wiring).
  Ships the LLM substrate as callable: real `LlmProvider`
  trait + 3 provider impls (Anthropic / OpenAI-compat /
  Ollama) + retry helper + Anthropic prompt-cache builder
  + `BudgetedProvider` decorator enforcing the $200/mo
  ceiling with auto-degrade at 80% (Q6 + Q11) + strict
  SQLite replay cache (D2 / Q8) + 9-row fixture cache + V9
  secrets grep + 3-provider × 3-role `llm-smoke` harness +
  two operator runbooks (`docs/runbooks/llm-{cost,replay}.md`).
  Q4 bonus rename **`cost::LlmProvider` enum → `ProviderKind`**
  (D1) freed the `LlmProvider` name for the trait; 5 call
  sites + serde wire shape preserved → zero on-disk ledger
  byte change. Q5d "Cache hit ratio" System Health row +
  Q11 denominator `$135 → $200` regenerated both
  `success-fixed-report-sample-{7d,90d}.md` bodies; tester
  re-locked the 2 corresponding anchors at T_FINAL
  (`520b1f29…` / `c656414e…`). The 9 strategy backtest
  anchors at `spec/anchors.toml:15-58` stay byte-identical
  (R14.2 / V8 enforced by T1937 negative-invariant — 11/11
  PASS).
  **Workflow shape**: 6 multi-pass developer cycles
  (`d0bcad2` → `c61afa5` → `441c136` → `f1dbe05` →
  `f1128e9` → `faaaec1`) over 2 days + tester gate
  (`8a41b47`). Two `[~]` partials flipped to `[x]` mid-
  cycle as their dependencies landed (T1912 audit-memo in
  pass 4; T1913 factory Research/Recording arms in pass 5).
  **44/45 dev tasks ticked** (T1938 cockpit "LLM budget"
  tile deferred to v2.1 + T1915 tracing-Layer half deferred
  + 2 pedantic clippy on `audit/src/query.rs:219,221` to
  v2.1 — all consolidated into `v2-llm-strategy-v21-followups`
  candidate). **1203 workspace tests passing, 0 failed.**
  **Unblocks**: Kronos v2.5 forecast overlay, Lumen Phase 6
  Assistant slot, reflection-memory follow-up briefs.
  Brief carries the architect-misdiagnosis-prevention
  workflow rules (Capability boundaries amendment from
  2026-05-12) only informally — the feature shipped on the
  pre-amendment single-tester model; future features apply
  the new test-runner/evaluator split.

- **UI test harness bootstrap (v0.1)** — shipped 2026-05-12
  (operator approval recorded as `[x] Approved — ship` in
  `spec/ui-test-harness-bootstrap/presentations/ui-test-harness-bootstrap-2026-05-12.md ## Approval`;
  evaluator `VERDICT → PASS` in
  `reports/evaluation-2026-05-12T13-15Z.md`).
  **First feature under the new `AGENT.md ## Capability boundaries`
  regime AND the first run of the test-runner / evaluator split.**
  Implemented week 1 of the 4-week dev-note adoption plan:
  `iced_test::screenshot` smoke test at three operator viewport
  slots (1280×720 / 1920×1080 / 3360×1890 @ 2.0); `image-compare`
  perceptual-diff forensics on snapshot failure; canvas hit-test
  grid sweep over every marker centroid at every viewport — closes
  detection-half of chart-canvas-overhaul V15; viewport-parametric
  helper on `dispatch_canvas_event_for_test`;
  `scripts/check_no_clocks_in_ui_tests.sh` determinism gate.
  Three baseline PNGs committed at
  `crates/ui/tests/visual-baselines/charts_screen_dark_{floor,typical,operator}.png`.
  **V8 PASS-with-H2-caveat**: render-half of chart-canvas-overhaul
  V15 (visible tooltip card in baseline) deferred to
  `ui-test-harness-canvas-state-seeding` candidate (Queue) per
  operator decision "Commit — V14 covered, V15 partial-accept"
  2026-05-12. New deps: `iced_test = 0.14.0`, `image-compare = 0.4`,
  `image = 0.25.6` (all dev-deps; zero production runtime impact).
  **Workflow meta-deliverables proven**: (a) architect's M0 API audit
  caught a load-bearing `iced_test::Snapshot::png()` assumption
  (method doesn't exist) before code was written; (b) developer
  caught a second `iced_test::screenshot → iced::window::Screenshot`
  API correction during M1 implementation and adjusted cleanly;
  (c) test-runner emitted raw log with honest `[~]` partial for
  4 sandbox-denied checks; (d) orchestrator supplemented those
  checks verbatim; (e) evaluator (read-only, fresh context,
  default-FAIL contract) emitted PASS with file:line cites for
  every V-item. Zero capability-boundary violations across all 6
  agent roles. Anchors PASS 11/11 byte-identical; 818 existing
  tests stay green; 8 net-new tests added; zero non-UI-crate
  changes. Weeks 2 / 3 / 4 follow-ups queued in
  `## Process / tooling`.

- **Chart canvas overhaul (v1.10.0)** — shipped 2026-05-12
  (operator approval recorded as `[x] Approved — ship` in
  `spec/chart-canvas-overhaul/presentations/chart-canvas-overhaul-2026-05-12.md ## Approval`;
  no overrides, no follow-up notes). Closes the six operator-
  reported items from the v1.9.0 retrospective: price axis on
  the LEFT gutter (USD labels), time axis on the BOTTOM gutter
  (HH:MM UTC), TradingView-style centering via
  `inner_rect_with_gutters`, top-right legend card
  (`PANEL_SUNKEN` fill + `BORDER_STRONG` outline at
  `crates/ui/src/widgets/chart_legend.rs:156-160`),
  viewer parity for `equity_curve` + `drawdown_band`, default
  window bump to 1920×1080 (min stays 1280×720), tooltip card
  clamp to inner-rect bounds. V15 (live tooltip-hover screenshot
  at 3360×1890) DEFERRED to the queued
  `ui-test-harness-bootstrap` v0.1 feature per operator decision
  D4 in `docs/dev-notes/archive/2026-Q2/ui-testing-direction-2026-05-12.md ## Section 9`
  — the first `iced_test::Simulator::snapshot().matches_image()`
  chart-hover test in that feature replaces the manual capture.
  Q4 local-time x-axis labels DEFERRED to v1.11
  `chart-x-axis-local-time` (shipped 2026-05-20, see Recent;
  UTC fallback shipped in v1.10.0 was the bridge). Retrospective surfaced the architect's
  "iced 0.14 canvas-scale bug" misdiagnosis (empirically
  disproved by orchestrator's red-rect + cyan-dot probe; T3002 /
  T3003 / T3007 / T3008 closed as no-op) — produced the
  `AGENT.md ## Capability boundaries` amendment (D5,
  load-bearing) + `docs/dev-notes/archive/2026-Q2/ui-testing-direction-2026-05-12.md`
  strategy document + `ui-test-harness-bootstrap` v0.1 follow-on
  feature now in Active. Anchor neutrality preserved: 11/11
  byte-identical (`bash scripts/verify_anchors.sh → ANCHORS PASS
  (11 / 11)`); zero changes to `crates/strategy/`, `crates/risk/`,
  `crates/backtest/`, `crates/reports/`, `crates/exec/`,
  `crates/audit/`, `crates/agent/`, `crates/core/`,
  `crates/reflection/`.

- **Chart buy/sell emphasis (v1.9.0)** — shipped 2026-05-11
  (operator verbal approval recorded as `[x] Approved — ship` in
  the presenter deck at
  `spec/chart-buy-sell-emphasis/presentations/chart-buy-sell-emphasis-2026-05-11.md`;
  no overrides, no follow-up notes). UI feature opened directly
  from operator visual feedback on the v1.8 cockpit — markers
  bigger + outlined + line-anchored + 6-field hover tooltip + click-
  through to existing journal_transaction_modal (R4.5 second
  consumer of the tape-row-audit-modal pattern); layered
  fills+signals ghost markers (R5 — signal source default-off via
  new `SignalLogConfig`); three counter views (cumulative-window
  volume tile + per-bar histogram + open-position mirror) above /
  below chart in Layout β; min window size 1280×720; Lumen window
  icon plumbing (macOS dock-icon limitation documented — needs
  `.app` bundle, candidate stub at
  `spec/cockpit-app-bundle/feature.md`).
  Anchor neutrality preserved: zero changes to `spec/anchors.toml`
  (11/11 byte-identical); zero modifications to `crates/strategy/`,
  `crates/risk/`, `crates/backtest/`, `crates/reports/`,
  `crates/exec/`. New additive surface: `crates/audit/migrations/009_strategy_signals.sql`
  + `audit::query::recent_signals` reader + `core::SignalView` type
  + `agent::SignalLogConfig { enabled: false }` + 2 new widgets
  (`chart_tooltip`, `volume_histogram`) + shared window-chrome helper
  (`window_icon`) + Lumen mark RGBA asset. Tester report:
  `spec/chart-buy-sell-emphasis/reports/test-2026-05-11-2103-chart-buy-sell-emphasis-final.md`
  (V1–V13 all PASS; 1000 / 0 / 4 tests across 144 binaries;
  11/11 anchors PASS). Brief:
  `spec/chart-buy-sell-emphasis/feature.md`
  (status: shipped, version: 1.9.0). Multi-cycle implementation
  arc: initial dev+ui-designer parallel ship + M6 follow-up
  (T2028–T2030) + M6.2 second follow-up (T2031–T2033) + hardening
  pass (corrected T2032 doc rationale + screenshot evidence). The
  iterative loop reflected headless-agent's inability to visually
  verify; addressed long-term by Screen Recording permission grant
  to the host IDE + documented screenshot-verification gate in
  M6.2 task bodies.
- **Reflection memory (v1.8.0)** — shipped 2026-05-10 (operator
  verbal approval recorded as `[x] Approve with notes` in the
  presenter deck at
  `spec/reflection-memory/presentations/reflection-memory-2026-05-08.md`;
  one note: flip `ReflectionConfig::enable_writer` default from
  `false` to `true` — applied in the same commit as approval).
  Replaces the R6 placeholder body in
  `crates/reports/src/render/memory_highlights.rs`
  with real reflection-memory output. New leaf crate
  `crates/reflection/` (lib only — types,
  regime + outcome classifiers, deterministic 32-dim embedding,
  post-mortem-analyst card generator, `ReflectionStore` trait + a
  `SqliteReflectionStore` linear-scan top-K impl, bounded mpsc
  writer task with Prometheus drop counter, retrieval API). Wired
  through `crates/agent/src/{config,main}.rs` + `crates/exec/src/paper.rs`.
  Re-locked the two `report-sample-*` anchors at
  `spec/anchors.toml:67-75`; the 9 strategy-backtest anchors at
  lines 15–58 are byte-identical (negative-invariant test t1812
  enforces). Q-resolutions: Q1 = Option A (deterministic v1, no
  LLM dependency); Q4 = report-only (Strategy trait unchanged);
  Q5 = distillation deferred to follow-up brief
  `reflection-memory-distillation`. Tester report:
  `spec/reflection-memory/reports/test-2026-05-08-2114-reflection-memory-final.md`
  (V1–V10 all PASS; 952 / 0 / 3 tests across 124 binaries; 11/11
  anchors PASS; cargo deny advisories/bans/licenses/sources all
  ok). Brief: `spec/reflection-memory/feature.md`
  (status: shipped, version: 1.8.0).
- **Presenter smoke test on `operator-success-reports`** — shipped
  2026-05-08 (operator verbal approval recorded as `[x] Approved —
  ship` in commit `587dad7`). Deck at
  `spec/operator-success-reports/presentations/operator-success-reports-2026-05-08.md`;
  pulled evidence from the archived final tester PASS (extracted
  from `spec/archive/pre-lumen-tester-reports-2026-04-to-05-03.tar.gz`)
  + a fresh `cargo test -p reports --test report_scenarios` re-run
  (4/4 PASS, body SHAs match anchors) + a fresh
  `scripts/verify_anchors.sh` PASS (11/11). Surfaced 4 smoke-test
  findings: (1) `present-results` skill missed the archive
  fallback for pre-Lumen tester reports — fixed in `8b139c2`;
  (2) `capture-screenshot` skill defaulted to a manual-capture
  instruction for non-UI features — fixed in `8b139c2` with a third
  "non-UI feature" branch; (3) backlog Recent section had stale
  relative paths inside link parens (cosmetic, fixed in `1a63156`);
  (4) confirmed the audit-immutability call on archived tester
  reports is correct (their internal `spec/features/...` /
  `spec/tasks/...` references describe the layout at time of
  writing). The presenter pipeline is now battle-tested before the
  next real-feature fire.

- **Lumen Phase 5 — HumanControl + AgentFeed rename** — shipped
  2026-05-07 (tester second-pass PASS, presenter approved
  2026-05-08). Tester reports at
  `spec/lumen-design-adoption/phase-5-humancontrol-agentfeed/reports/test-2026-05-07-lumen-phase-5-humancontrol-agentfeed.md`
  (first-pass FAIL on fmt drift) and
  `spec/lumen-design-adoption/phase-5-humancontrol-agentfeed/reports/test-2026-05-07b-lumen-phase-5-humancontrol-agentfeed.md`
  (second-pass PASS); brief at
  `features/lumen-phase-5-humancontrol-agentfeed.md`
  (status: `shipped`); presenter deck at
  `spec/lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md`.
  **First phase to ship net-new operator-write surfaces since v0**:
  HumanControl panel widget (execution-mode segmented control +
  daily loss limit / max position / used-today P&L mirror rows +
  kill button as bottom action) on a new "Control" sidebar entry
  (7th); single-click pause-strategy toggle on Strategies-detail;
  typed-confirm `OVERRIDE` flow for risk-veto override per surfaced
  veto event. Two new audit writers (`strategy_paused`,
  `risk_veto_overridden`) — additive `StrategyEventKind` variants
  with no SQL migration (column already TEXT). Module rename
  `tape` → `agent_feed` via `mv` + git rename detection;
  `Cockpit::tape` field name preserved (Q14) to avoid 100+ test
  ripple. **TD-1 four-phase deferral CLOSED** via Path (b) —
  `crates/ui/src/widgets/focus_ring.rs` Subscription-driven
  custom-widget escape hatch wraps all four destructive surfaces
  with a visible accent-bordered halo on focus. New TD-2 row
  added: risk-engine veto-emit upstream wiring deferred (Phase 5
  ships override surface over an empty live `Vec<VetoEvent>`;
  not a safety primary, an observability gap). Anchor risk: zero
  — verified PASS at ship (11/11 byte-identical post additive
  audit-writer additions). 896 tests passed across 110 binaries
  (46 + 2 net-new vs Phase 4); rust-validate clean (after one-line
  `cargo fmt --all` fixup between tester passes); 86 baselines
  attested by ui-designer (67 panel + 17 widget + 2 audit; 13
  net-new + 9 renamed); R16.3 brand-bleed grep clean. Architect
  ratified 15/15 Q-items with zero principled overrides.
  **Phase 5 is the last shippable phase of the lumen-design-adoption
  initiative absent v2 LLM** — Phase 6 (Assistant slot) is reserved
  until the v2 LLM strategy lands.

- **Lumen Phase 4 — Backtest panel (`viewer` bin)** — shipped
  2026-05-06 (tester second-pass PASS, presenter approved
  2026-05-06). Tester reports at
  `spec/lumen-design-adoption/phase-4-backtest-panel/reports/test-2026-05-06-lumen-phase-4-backtest-panel.md`
  (first-pass FAIL on `clippy::match_same_arms`) and
  `spec/lumen-design-adoption/phase-4-backtest-panel/reports/test-2026-05-06b-lumen-phase-4-backtest-panel.md`
  (second-pass PASS); brief at
  `features/lumen-phase-4-backtest-panel.md`
  (status: `shipped`); presenter deck at
  `spec/lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md`.
  Adds the new `viewer` binary at `crates/ui/src/bin/viewer.rs`
  (workspace now ships 3 bins), KPI strip + equity curve + drawdown
  band widgets sharing a refactored `widgets::canvas_chart` core
  with Phase 2's price chart, the cross-phase `core::EquitySeries`
  primitive (rich struct with precomputed drawdown vector inside
  `EquityPoint`), additive `audit::query::equity_curve_for_strategy(strategy_id, since, until)`
  sibling of Phase 2/3 filtered queries, markdown summary parser
  with graceful "—" fallback for missing fields (the 11 anchored
  sample reports omit CAGR + Win rate by design), and **closes the
  Phase 3 deferral** (Strategies-detail sparkline placeholder
  retires; real `widgets::sparkline` lands fed by the new audit
  query). Anchor risk: zero — verified PASS at ship (11/11 byte-
  identical, viewer reads existing committed reports). 850 tests
  passed across 108 binaries (40 + 4 net-new vs Phase 3); rust-
  validate clean (after orchestrator's one-line `match_same_arms`
  fix between tester passes); 72 baselines attested by ui-designer;
  R16.3 brand-bleed grep clean. Architect ratified 12/12 Q-items
  with zero principled overrides (Q1 shape refinement: drawdown_pct
  nested inside `EquityPoint` rather than parallel Vec — eliminates
  length-coupling). **Phase 5 inherits the TD-1 tightening point**:
  Phase 5 ships net-new operator-write controls with typed-confirm
  flows, making the focus-ring deferral (iced still pins `=0.14.0`)
  load-bearing — Phase 5 either folds the iced 0.15+ upgrade or
  commits to the custom-widget escape hatch.

- **Lumen Phase 3 — Detail screens (Strategies / Risk / Audit)** —
  shipped 2026-05-05 (tester first-pass PASS, presenter approved
  2026-05-06). Tester report at
  `spec/lumen-design-adoption/phase-3-detail-screens/reports/test-2026-05-05-lumen-phase-3-detail-screens.md`;
  brief at
  `features/lumen-phase-3-detail-screens.md`
  (status: `shipped`); presenter deck at
  `spec/lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md`.
  Adds Strategies / Risk / Audit sidebar entries (3 → 6) + per-screen
  detail bodies, additive `008_journal_transactions_venue.sql`
  migration (default `'binance'` backfill), `post_fill` writer's
  new `venue: Venue` parameter wired across ~25 call-sites,
  `RiskTelemetry` channel on `agent::EventBus` mirroring `MarketHealth`,
  sibling audit query `recent_journal_filtered` (additive to
  `recent_fills_filtered`), kill-threshold proximity gauge with
  tri-band ramp (`UP_500` ≤70% → `WARN_500` >70% → `DOWN_500` >90%),
  Audit screen filter chip-row (venue · symbol · kind · time-range)
  + fixed 250-row pagination + reuse of T1208 modal, cross-link
  Home → Strategies-detail. Two developer passes (pass 1 cut at
  clean tick boundary after T1701 + T1703 due to context budget;
  pass 2 ticked T1702 migration + T1704–T1716). Architect ratified
  11/11 Q-items with one deferral (Q6 equity-since-deploy sparkline
  → Phase 4, since the cheap path doesn't exist on the current
  state shape and Phase 4 needs the same equity-history primitive).
  Anchor risk: zero — verified PASS at ship (11/11 byte-identical
  post-migration, verified twice during dev pass + once at tester
  gate). 810 tests passed across 104 binaries (29 + 6 net-new vs
  Phase 2); rust-validate clean (fmt + clippy `-D warnings` +
  cargo-deny + docs); 65 baselines attested by ui-designer
  (zero `unknown` token escapes); R16.3 brand-bleed grep clean.

- **Lumen Phase 2 — Shell IA + Charts** — shipped 2026-05-05.
  Tester first-pass `VERDICT → PASS`; report at
  `spec/lumen-design-adoption/phase-2-shell-ia-charts/reports/test-2026-05-05-lumen-phase-2-shell-ia-charts.md`.
  Brief at
  `features/lumen-phase-2-shell-ia-charts.md`
  (status: `shipped`). Presenter deck approved by operator at
  `spec/lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md`.
  Adds left-sidebar shell (fixed 180 px, T1507-styled, no icons),
  Screen routing (`Cockpit::current_screen` × six variants — Home /
  Debug / Charts wired in Phase 2; Strategies / Risk / Audit
  declared for Phase 3), Home screen (Phase 1 widgets re-housed),
  Debug screen (kill / latency / market health / server time /
  version / placeholder logs), Charts screen (chip-row symbol
  selector + canvas line-series price plot + buy/sell triangle
  markers from `recent_fills_filtered`), per-`(Venue, Symbol)`
  rolling `ChartBuffer` (cap 60 1-min bars), live mode via existing
  `bars_tx`, fixtures mode via deterministic `synthetic_candles`,
  additive `audit::query::recent_fills_filtered(venue, symbol,
  since, until)`, right-rail Phase 6 Assistant slot reservation
  (`Length::Fixed(0.0)`). Anchor risk: zero — verified PASS at
  ship (11/11 anchors byte-identical). 781 tests passed across 98
  binaries (24 + 2 net-new vs Phase 1); rust-validate clean
  (fmt + clippy `-D warnings` + cargo-deny + docs); 53 baselines
  attested by ui-designer (zero `unknown` token escapes); R16.3
  brand-bleed grep clean. All 11 architect Q-resolutions ratified
  with zero deviations from analyst recommendation. **Phase 3
  prerequisite carried forward**: additive `journal_transactions.venue`
  column migration needed before non-Binance fills can populate
  the chart's marker query.

- **Lumen Phase 1 — Foundation (tokens + tiers + status bar)** —
  shipped 2026-05-04. Tester third-pass `VERDICT → PASS`; report at
  `spec/lumen-design-adoption/phase-1-foundation/reports/test-2026-05-04c-lumen-phase-1-foundation.md`.
  Brief at
  `features/lumen-phase-1-foundation.md`
  (status: `shipped`). Replaced the 12-token theme with the full
  Lumen palette (warm + cool neutrals, accent ramp, sage / clay /
  warn / info semantics, both light and dark modes); added Tier
  0/1/2/3 elevation surface tokens; added whisper shadows + sunken
  inset; added focus ring (Q11 / TD-1 deviation: hover-state ring
  on buttons + ACCENT border-shift on focused inputs as bounded
  approximation, two named upgrade triggers); extended spacing /
  radii / typography ladders to the full Lumen scale; added motion
  tokens; applied Tier 1 styling to existing 6 widgets; applied
  sunken styling to the kill-confirm input; applied the active-row
  2 px left rule to tabular widgets; added a new
  `widgets::status_bar` widget rendering connection / latency /
  account / server time always-visible at the bottom of the shell;
  refreshed the existing 36 panel snapshot baselines (5 net-new for
  T1506 / T1508 = 41 total); superseded
  `docs/ui-design-principles.md` with a
  Lumen-anchored rewrite. Anchor risk: zero — verified PASS at
  ship (11/11 anchors byte-identical). 757 tests passed across 96
  binaries; rust-validate clean (fmt + clippy + cargo-deny + docs);
  R16.3 brand-bleed grep clean. Unblocked the 2026-05-04 master
  roadmap revision (4 → 6 phases).
- **v1.5b multi-venue + 1s aggregated trades** — shipped 2026-05-03.
  Brief at
  `features/v1-5b-multi-venue.md`.
  Coinbase + Kraken adapters, USDC pair mirror set (10 symbols),
  T612 multi-symbol live `BinanceFeed`, 1 s aggregated trades,
  `Venue` enum on `Tick` / `Bar`, per-venue feed-reconnect
  provenance. Plumbing-only — expanded the data side, not the
  execution side. 15 R-items, 12 V-items, 12 open questions
  resolved. Anchor risk: zero by construction (no `venue` strings
  in any committed report body). Closes v1.5a Q5 (USDC pairs
  blocker) and v1 closeout T612. The cockpit / `cockpit_live` is
  now stable on the data side; this is the clean window the
  Lumen design adoption initiative lands into.

