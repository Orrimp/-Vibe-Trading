---
title: Test Report
feature: live-equity-history-durable
run_id: 2026-06-11-1200-UTC
commit: fc8c9637df3fa5472c4c7d95fc70fe3e7e317e33 (uncommitted working tree)
agent: tester
verdict: PASS
---

# Test Report — live-equity-history-durable — 2026-06-11 12:00 UTC

## 1. Scope

- **Feature / change under test:** Durable live equity history — persist per-bar equity snapshots in paper/live mode to the audit SQLite ledger (`equity_snapshots` table, migration 013), and hydrate the cockpit Live screen's equity curve + KPI strip on boot. Research-replay mode persists nothing. ADR-0052.
- **Spec refs:** `spec/live-equity-history-durable/feature.md`, `spec/live-equity-history-durable/tasks.md`, `spec/architecture/adr/0052-durable-live-equity-series.md`
- **Commit SHA:** `fc8c9637df3fa5472c4c7d95fc70fe3e7e317e33` (HEAD); implementation delivered as uncommitted working-tree modifications to `crates/audit`, `crates/agent`, `crates/ui`.
- **Rust toolchain:** `rustc 1.94.1 (e408947bf 2026-03-25)`
- **OS / arch:** Darwin 25.5.0 (macOS, arm64)

## 2. Static Analysis

| Check              | Result | Notes |
|--------------------|--------|-------|
| `cargo fmt --check -p audit -p agent` | PASS | No diffs. |
| `cargo fmt --check -p ui` | PASS | No diffs. Pre-existing unformatted `crates/ui/benches/chart_build_probe.rs` excluded per brief (known tech-debt). |
| `cargo clippy -p audit -p agent -p ui` | PASS (new code) | 16 pre-existing warnings across `ui` lib/bin targets (deprecated Screen variants in tests, `must_use`, non_snake_case in lab/date_range tests, collapsible-if in cockpit_live). Zero warnings attributable to the new files: `equity_store.rs`, migration `013_equity_snapshots.sql`, the `post_equity_snapshot`/`equity_snapshot_tail`/`purge_old_equity_snapshots` additions in `journal.rs`/`query.rs`, `reconciler.rs` and `runtime.rs` additions, `state.rs` `PnlHydrated` arm, `cockpit_live.rs` hydrate task, `live.rs` seam helper. |
| `cargo audit` | not run | Per brief — focused T10 matrix only. |
| `cargo deny` | not run | Per brief — focused T10 matrix only. |

## 3. Unit & Integration Tests

### AC1 — Paper mode persists one row per bar

**Test:** `crates/agent/tests/equity_store_integration.rs::ac1_paper_mode_persists_one_row_per_bar`

Command: `cargo test -p agent -- ac1_paper_mode_persists_one_row_per_bar`

Result: **PASS**

### AC2 — Research mode persists zero rows

**Test:** `crates/agent/tests/equity_store_integration.rs::ac2_research_mode_writes_zero_rows`

Command: `cargo test -p agent -- ac2_research_mode_writes_zero_rows`

Result: **PASS**

### AC3 — Writer/reader round-trip (in-memory Ledger)

**Tests:**
- `crates/audit/src/query.rs::tests::equity_snapshot_round_trip_ac3`
- `crates/audit/src/query.rs::tests::equity_snapshot_tail_limit_ac3`
- `crates/audit/src/query.rs::tests::equity_snapshot_tail_empty_table`

Command: `cargo test -p audit -- equity_snapshot_round_trip_ac3 equity_snapshot_tail_limit_ac3`

Result: **PASS** — Decimal-as-TEXT + RFC3339-micros round-trip lossless; monotone `bar_ts` order preserved; LIMIT honored.

### AC4 — Hydration seeds the buffer (model layer)

**Tests:**
- `crates/ui/src/state.rs::tests::pnl_hydrated_seeds_buffer_curve_and_strip_ready` — M≥2 rows → buffer seeded, curve Ready, KPI strip Ready, before any live tick.
- `crates/ui/src/state.rs::tests::pnl_hydrated_one_row_curve_ready_strip_loading` — 1 row: curve Ready (≥1 pts), KPI strip Loading (≥2-pt rule).
- `crates/ui/src/state.rs::tests::pnl_hydrated_respects_buffer_cap` — LIMIT 2880 honored.
- `crates/ui/src/state.rs::tests::pnl_hydrated_empty_is_noop` — empty tail is a no-op.

Command: `cargo test -p ui --lib -- pnl_hydrated`

Result: **PASS**

### AC5 — Post-hydrate live append still lands (model layer + render layer)

**Model-layer test:** `crates/ui/src/state.rs::tests::live_append_after_hydrate_lands_not_dropped` — after `PnlHydrated`, one live `PnlRefreshed(now())` appends (not dropped by the `as_of` delivery guard).

**Render-layer test (pixel gate):** `crates/ui/tests/live_equity_render.rs::live_append_after_hydrate_still_renders_and_grows` — hydrate from a 2025 `as_of` tail, then deliver a `PnlRefreshed(Timestamp::now())`. Asserts: buffer grows by 1, curve renders ≥ `CURVE_DREW_MIN_ACCENT` ACCENT pixels, x-span ≥ `CURVE_X_SPAN_MIN`, and x-span ≥ the hydrated-only baseline (forward bar extended the time axis).

Command: `cargo test -p ui --features live --test live_equity_render -- live_append_after_hydrate_still_renders_and_grows`

Result: **PASS**

### AC6 — Hydrated boot renders at the pixel layer (THE gate, R5)

**Test:** `crates/ui/tests/live_equity_render.rs::hydrated_boot_curve_actually_renders`

Drive ONE `Message::PnlHydrated(faked 8-row tail)` — zero `PnlRefreshed`. Assert: buffer.len() == 8, curve Ready, `live_equity_hydrated == true`, rendered polyline ≥ `CURVE_DREW_MIN_ACCENT` (200) ACCENT pixels, x-span ≥ `CURVE_X_SPAN_MIN` (400 px). A model-Ready-but-blank-canvas regression fails here.

Command: `cargo test -p ui --features live --test live_equity_render -- hydrated_boot_curve_actually_renders`

Result: **PASS**

Full live_equity_render suite (7/7):

| Test | Result |
|------|--------|
| `diag_accent_bounding_box` | PASS |
| `live_equity_curve_actually_renders` | PASS |
| `harness_catches_dropped_points_empty_curve` | PASS |
| `healthy_curve_draws_far_more_than_broken` | PASS |
| `hydrated_boot_curve_actually_renders` (AC6) | PASS |
| `live_append_after_hydrate_still_renders_and_grows` (AC5) | PASS |
| `flat_and_single_point_curves_render_without_panic` | PASS |

### AC7 — Anchor-safe migration (verify_anchors.sh)

Command: `bash scripts/verify_anchors.sh`

Result: **PASS — 119 / 119 anchors byte-unchanged.**

Migration `013_equity_snapshots.sql` is purely additive (`CREATE TABLE IF NOT EXISTS`, no `ALTER`, no backfill, no `UPDATE`). The backtest binary instantiates the reconciler with `bus = None` and never writes to `equity_snapshots`. All 119 body-SHA-256 backtest report anchors are byte-unchanged by construction (ADR-0052 § A3 / anchor-safety proof).

### AC8 — Retention is bounded (purge test)

**Tests:**
- `crates/audit/src/query.rs::tests::equity_snapshot_purge_ac8` — inserts rows past + within the retention horizon, verifies only within-horizon rows survive after `purge_old_equity_snapshots`.

Command: `cargo test -p audit -- equity_snapshot_purge_ac8`

Result: **PASS**

### AC9 — Fixtures cockpit smoke + every I/O behind a trait

**Build gate:** `cargo build -p ui --bin cockpit --features fixtures` — **PASS** (no `live` feature, no hydrate task compiled in, no `equity_store` dep; `Message::PnlHydrated` arm is compiled in unconditionally since it is a state.rs model method, but no hydrate is issued without the `live` feature).

**Cfg-gate by inspection:** `crates/ui/src/bin/cockpit_live.rs:816` — the `equity_hydrate_task` is guarded `#[cfg(feature = "live")]`; the `cockpit` fixtures binary does NOT compile with `--features live`, so the hydrate task is `Task::none()` by the `#[cfg(not(feature = "live"))]` fallback at line 847. The `equity_store` in the agent-side `RunHandles` is `None` in research mode — confirmed by inspection at `crates/agent/src/main.rs:317-325` and `crates/ui/src/bin/cockpit_live.rs:483-486`.

**Trait boundary:** the durable store is reached exclusively through `audit::LiveEquityStore` (`crates/audit/src/equity_store.rs`). Production impl is `LedgerEquityStore` (wraps `Arc<Ledger>`). Tests use `FakeLiveEquityStore`. No direct sqlx in `crates/ui` — the hydrate calls `audit::query::equity_snapshot_tail` via an `Arc<Ledger>` reference held in `AppState`. Confirmed: `ui`'s no-sqlx edge is intact.

**New dep check:** `crates/audit/Cargo.toml:24` — `async-trait.workspace = true`. The workspace `Cargo.toml:72` already carries `async-trait = { version = "0.1" }`. **Zero new dependencies introduced.**

**Seam test (T7):** `crates/ui/src/lib.rs::live::tests::equity_hydrate_gate_issued_in_paper_skipped_in_research` — verified by `cargo test -p ui --features live --lib`.

Result: **PASS**

### T7-contract seam test

`live::tests::equity_hydrate_gate_issued_in_paper_skipped_in_research` — verified in `cargo test -p ui --features live --lib` (447/447).

Result: **PASS**

### Full suite counts

| Crate / suite | Passed | Failed | Ignored | Duration |
|---------------|-------:|-------:|--------:|--------:|
| `audit` (lib + 36 integration tests) | 139 | 0 | 1 (doc) | ~2 s |
| `agent` (lib + 18 integration tests) | 119 | 0 | 3 | ~5 s |
| `ui --lib` | 447 | 0 | 0 | ~0.7 s |
| `ui --features live --lib` | 447 | 0 | 0 | ~0.7 s |
| `ui --test live_equity_render` | 7 | 0 | 0 | ~0.4 s |
| `ui --test panel_snapshots` | 103 | 0 | 0 | ~0.3 s |
| **Total (new-feature crates)** | **1262** | **0** | **4** | |

### Failing Tests

_none_ — all tests in the new-feature crates pass.

## 4. Property / Fuzz Tests

_n/a_ — no proptest/cargo-fuzz suites added in this feature. The `FakeLiveEquityStore` exercises the monotone sort + limit invariants via deterministic unit tests.

## 5. Backtest Results

_n/a_ — this is a read-only monitor persistence feature. No strategy logic, no sizing math, no decision variable. The baseline-equity-divergence e2e gate explicitly does NOT apply (A3, stated in `feature.md` § Not a strategy or sizing feature). The backtest reconciler uses `bus = None` and never writes to `equity_snapshots` — anchor-safety proven by construction and verified by `verify_anchors.sh` (119/119 PASS).

## 6. Benchmarks

_n/a_ — no hot paths touched. Per-bar fire-and-forget SQLite write at minute-bar cadence (A6); no latency regression possible.

## 7. Composition Review

### Mode gate — both binaries

**Headless `trading` bin (`crates/agent/src/main.rs:317-325`):**
```
let equity_store: Option<Arc<dyn audit::LiveEquityStore>> =
    if cfg.mode != agent::config::Mode::Research {
        Some(Arc::new(audit::LedgerEquityStore::new(Arc::clone(&ledger))))
    } else {
        None
    };
```
In Research mode: `None` → `RunHandles.equity_store = None` → `reconciler.rs` fire-and-forget branch is skipped. In paper mode: `Some(LedgerEquityStore)` → per-bar write fires.

**cockpit_live (`crates/ui/src/bin/cockpit_live.rs:483-486` for store; `:816` for hydrate gate):**
```
let equity_store = if cfg.mode == Mode::Research { None } else { Some(Arc::new(...)) };
// ...
#[cfg(feature = "live")]
let equity_hydrate_task = if !ui::live::should_hydrate_equity_on_boot(&boot_mode) {
    iced::Task::none()
} else { /* boot query */ };
```
Research mode: `equity_store = None` (no write) AND hydrate task = `Task::none()` (no hydrate). Paper mode: store present + hydrate task fires.

Both binaries gate correctly. The write-and-hydrate invariant is: research = session-scoped (no write, no hydrate); paper/live = durable (write per bar, hydrate on boot).

## 8. Pre-existing reds (NOT this feature — verified identical failure text)

| Test | Failure | Status |
|------|---------|--------|
| `ui::lab_run_engine::inner::h3_in_memory_equals_cached_disk` | Panics: `write_report=true should produce a report_path` — hardcodes `XRPUSDT`, data absent locally. | Pre-existing, unchanged. |
| `backtest::scenarios::montecarlo::tests::run_path_funding_none_is_anchor_neutral` | Flaky — passed in second run. | Pre-existing flakiness, not a new failure. |
| `backtest::scenarios::montecarlo::tests::run_path_k_short_zero_byte_identical_to_head` | Flaky — passed in second run. | Pre-existing flakiness, not a new failure. |

All backtest crate tests passed clean on the second run (76/76 passed, 5 ignored). The lab_run_engine failure is the only deterministic pre-existing red.

## 9. Baseline-equity-divergence e2e gate — N/A (explicit)

This feature is a **read-only monitor persistence** feature: no strategy overlay, no sizing modifier, no decision variable. Per CLAUDE.md the baseline-equity-divergence e2e gate applies only to strategy overlays / sizing modifiers. This is stated explicitly in `feature.md` § "Not a strategy or sizing feature" and in `tasks.md` T10 note. **Gate does NOT apply; not filed as missing.**

## 10. Spec-lint gate

Command: `python3 scripts/spec_lint.py`

Result: **spec-lint: FAIL (71 violations in 2 categories)** — `dead-link (66)`, `trace-broken-path (5)`.

Comparison to baseline (`spec/dev-notes/audit-2026-06-08.md`): baseline was 94 violations (87 dead-link + 7 trace-broken-path). Current is 71 violations (66 dead-link + 5 trace-broken-path) — **FEWER than baseline on both categories** (−21 dead-link, −2 trace-broken-path). This is an improvement, not a regression, caused by the Phase 1+2 cleanup commit `1405042`. All violations are pre-existing; none are attributable to this feature.

Per tester protocol: new regressions (category counts grew) block PASS; pre-existing baseline violations do not. No new category introduced; counts decreased. **Does not block PASS.**

## 11. Pre-existing spec debt (carried from audit-2026-06-08)

All 71 violations are pre-existing. Categories:
- **dead-link (66):** stale links in archived/historical feature specs, ADRs referencing cleaned-up paths (v25-kronos, crates/forecast/src/bin, /tmp/orch-diag screenshots, etc.). Fewer than baseline by 21 — cleanup removed some.
- **trace-broken-path (5):** `REQ-LAB-YAHOO-REALDATA-V0-1-4-001`, `REQ-VISUAL-FAIL-HTML-REPORTER-001` (2 paths), `REQ-QUEUE-STALENESS-RECONCILIATION-001`, `REQ-OPERATOR-LEDGER-SCHEMA-LINT-001`. Fewer than baseline by 2.

## 12. AC7 Anchor verification — explicit count

`bash scripts/verify_anchors.sh` output: `ANCHORS PASS  (119 / 119)`

All 119 body-SHA-256 anchors are byte-unchanged. The `013_equity_snapshots.sql` migration is additive by construction; no backtest report file was touched.

## 13. Verdict

**`PASS`**

All 9 acceptance criteria are green. The full new-feature test suite ran 1262 tests with zero failures. The render-layer gate (AC6) and the delivery-guard reconciliation (AC5) both pass at the rasterized pixel layer using `iced_test::screenshot`. The anchor count is 119/119 byte-unchanged. The fixtures cockpit builds clean with no hydrate path. The mode gate is verified by inspection at both binary entry points and by the `equity_hydrate_gate_issued_in_paper_skipped_in_research` seam test. No new deps introduced. Spec-lint counts decreased vs. baseline (improvement). The baseline-equity-divergence e2e gate is N/A by explicit spec decision.

## 14. Routing

`VERDICT → PASS` — all AC1–AC9 green; 119/119 anchors; ready to merge/ship.
