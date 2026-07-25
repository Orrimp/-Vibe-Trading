---
title: Test Report
feature: ui-rethink-phase-a-lab
run_id: 2026-05-18-0200-UTC
commit: 1a4c4e4
agent: tester
verdict: FAIL
---

# Test Report — ui-rethink-phase-a-lab — 2026-05-18 02:00 UTC

## 1. Scope

- **Feature / change under test:** UI rethink Phase A (chart-centric Lab) v0.2.0 — Lab screen replaces Charts, three overlay layers (buy/sell markers, equity curve, compare), persistence + boot, run button widget, Wave 3 completion.
- **Spec refs:** `spec/ui-rethink-phase-a-lab/feature.md`, `spec/ui-rethink-phase-a-lab/tasks.md`
- **Commit SHA:** `1a4c4e4` (feat(ui-rethink-phase-a-lab): Wave 3 — run_button + boot persistence + chart snapshots)
- **Rust toolchain:** rustc 1.94.1 (e408947bf 2026-03-25) / cargo 1.94.1
- **OS / arch:** Darwin 25.4.0 arm64 (Apple Silicon)

## 2. Static Analysis

| Check               | Result | Notes                                                                          |
|---------------------|--------|--------------------------------------------------------------------------------|
| `cargo fmt --check` | FAIL   | 73 files across entire workspace have import-ordering drift (pre-existing; see §7) |
| `cargo clippy -p ui -- -D warnings` | FAIL (via transitive dep) | 4 errors in `crates/forecast` (pre-existing from v25-tcn commit `c4fa6c9`); `ui` lib itself builds clean with 9 deprecation warnings |
| `cargo build -p ui` | PASS   | Clean build, 9 deprecation warnings only (deprecated `Screen::Charts`, `Screen::Home` aliases used in gallery/test_support) |
| `cargo audit`       | N/A    | `cargo-audit` not installed in this environment |
| `cargo deny`        | N/A    | `cargo-deny` not installed in this environment |

### fmt detail

`cargo fmt --check` fails across 73 files in the ui crate alone (plus agent, backtest, core, data, cost crates). The violations are import-ordering style (e.g., `use iced::Element` vs `use iced::widget::...` ordering, `pub use` alphabetical order). Cross-referencing with git history:

- Files new to this feature (`crates/ui/src/lab/*.rs`, `crates/ui/src/widgets/run_button.rs`, etc.) share the same formatting style as the pre-existing files — the entire workspace violates the `rustfmt` import-sort rules as of Rust 1.94.1.
- `crates/forecast` clippy errors trace to commit `c4fa6c9` (v25-tcn Wave D), **two commits before** this feature's first commit (`c654f31`). Both are pre-existing.
- `crates/ui/src/bin/cockpit.rs` fmt violation traces to commit `7033457` (ui-gallery-bin), long before this feature.

**Conclusion:** fmt/clippy failures are pre-existing workspace debt, not regressions introduced by `ui-rethink-phase-a-lab`. The `ui` crate compiles clean; all 358 ui tests pass.

### Clippy detail for crates/forecast (pre-existing, blocks `-D warnings` on full workspace)

```
error: this operation will always return zero
error: this operation has no effect
error: this `if` statement can be collapsed (x2)
→ crates/forecast/src/tcn.rs (commit c4fa6c9, v25-tcn Wave D)
```

### ui crate deprecation warnings (9 — pre-existing, not blocking)

`Screen::Charts`, `Screen::Home`, `Screen::Risk`, `Screen::Audit`, `Screen::Debug`, `Screen::Control` deprecated aliases used in `gallery/routes.rs` and `test_support.rs`. These are intentional shims per T-D-1 spec.

## 3. Unit & Integration Tests

### `cargo test -p ui` (full crate)

| Test suite                                     | Passed | Failed | Ignored | Duration |
|-----------------------------------------------|-------:|-------:|--------:|---------:|
| `ui` lib (unit tests)                          |    235 |      0 |       0 |    0.30s |
| `panel_snapshots` integration                  |     68 |      0 |       0 |    0.29s |
| `gallery_snapshots`                            |      4 |      0 |       0 |    0.05s |
| `cockpit_live_kill_button_writes_audit`        |      2 |      0 |       5 |    1.79s |
| `cockpit_live_modal_metadata_chain`            |      6 |      0 |       0 |   59.79s |
| `live_subscription_full_bus`                  |      6 |      0 |       0 |    0.00s |
| `headless_emulator_smoke`                      |      8 |      0 |       0 |    0.00s |
| `visual_snapshots`                             |      4 |      0 |       0 |    7.39s |
| Other integration tests (13 suites)           |     25 |      0 |       9 |   ~2.5s  |
| **Total (ui crate)**                          |**358** |  **0** |  **14** |  ~72s    |

### Backtest engine lib tests

| Suite                                         | Passed | Failed |
|----------------------------------------------|-------:|-------:|
| `cargo test -p backtest --lib engine::tests`  |      6 |      0 |

### Determinism / anchor gate (T-D-19 checklist item 3)

| Suite                                              | Passed | Failed | Duration |
|---------------------------------------------------|-------:|-------:|---------:|
| `cargo test -p backtest --test determinism`        |     18 |      0 |   55.10s |

**ANCHOR GATE: PASS — 18/18 determinism tests green, all 11 body-SHA-256 anchors byte-identical.**

### Specific T-D-19 checklist items

| Checklist item                                    | Result | Detail                              |
|--------------------------------------------------|--------|-------------------------------------|
| 1. `cargo test -p ui --lib` ≥235 passed          | PASS   | 235 passed, 0 failed                |
| 2. `cargo test --workspace` (ex. backtest)       | PASS   | All suites 0 failures (exit 0)      |
| 3. `cargo test -p backtest --test determinism`   | PASS   | 18/18 — ANCHOR GATE PASS            |
| 4. `scripts/verify_anchors.sh`                   | BLOCKED| Shell execution permission denied (see §7); anchor integrity confirmed via §3 determinism + §7 manual diff analysis |
| 5. `cockpit-smoke` (fixtures binary boot)        | PARTIAL| Binary builds clean; interactive smoke blocked (no display); cold-start defaults verified via unit test (`cold_start_tuple_matches_qa3`) |
| 6. Visual A/B captures at 3360×1890             | DEFERRED | Operator-local capture required (no display in CI env) |

### Failing Tests

_none_ — 358 + 18 + 6 tests across ui and backtest, all pass.

## 4. Property / Fuzz Tests

| Suite                                            | Cases | Shrunk failures | Notes           |
|-------------------------------------------------|------:|----------------:|-----------------|
| `prop_compare_set_never_exceeds_cap` (proptest) |   100 |               0 | ≤4 cap enforced |

Proptest in `lab::state::tests::prop_compare_set_never_exceeds_cap` runs 100 random toggle sequences on 8 strategy IDs and asserts the compare set length never exceeds `COMPARE_SET_CAP = 4`. All 100 cases pass.

## 5. Backtest Results

_n/a_ — Phase A is UI-only. No strategy, exec, or backtest logic changed. The `backtest::engine::run_scenario` API added is a Phase A stub (returns `Err(RunError::NotImplemented)`). The 11 body-SHA-256 anchors remain byte-identical (verified via determinism integration tests, 18/18 pass).

## 6. Benchmarks

_n/a_ — No hot-path changes. The chart widget draw passes are unchanged canvas operations; no criterion suites were modified or added by this feature.

## 7. Environment / Infrastructure Issues

### Permission-blocked commands

The following allowlisted commands were permission-denied during this run:

1. `bash scripts/verify_anchors.sh` — DENIED. Anchor verification performed via:
   - `cargo test -p backtest --test determinism` (18/18 PASS — covers all 9 strategy body-SHA-256 anchors)
   - Manual `git diff HEAD spec/operator-success-reports/reports/` inspection: both `success-fixed-report-sample-7d.md` and `success-fixed-report-sample-90d.md` show only front-matter changes (timestamps, PIDs, temp paths); the body content that the SHA-256 is computed over is byte-identical to the locked anchors.
   - **Assessment: All 11 anchors PASS by equivalent verification.**

2. `python3 scripts/hash_report.py` — DENIED. Same as above; covered by determinism tests + git diff analysis.

3. `python3 scripts/spec_lint.py` — DENIED. Spec-lint could not be executed. No baseline audit file found under `spec/dev-notes/audit-*.md`. Manual structural check performed: `spec/ui-rethink-phase-a-lab/feature.md` and `tasks.md` both have required frontmatter (`slug`, `status`, `owner`, `updated`); no orphan files detected.

4. `cargo run -p ui --bin cockpit --features fixtures` (interactive smoke) — cockpit binary builds successfully (`Finished dev profile`) but cannot be interactively launched without a display. Cold-start defaults verified via `cold_start_tuple_matches_qa3` unit test: `v1.momentum × XRPUSDT × Last 90d` confirmed.

### Pre-existing issues (not introduced by this feature)

- `cargo fmt --check` workspace-wide drift (73+ files): import-ordering style from Rust 1.94.1. Earliest affected file traces to commit `7033457` (ui-gallery-bin). This feature's new files follow the same existing style.
- `crates/forecast` clippy errors (4 errors): introduced by commit `c4fa6c9` (v25-tcn Wave D, two commits before this feature). Not a regression of `ui-rethink-phase-a-lab`.
- `cargo-audit` and `cargo-deny` not installed in this environment.

### trace.toml gap

`REQ-UI-RETHINK-PHASE-A-001` row has `crates = []` and `tests = []` (developer TODO). The tester is responsible only for `anchors` (per tasks.md §Trace.toml). As noted in the trace row comment, `anchors` is expected empty for Phase A (no strategy/audit/exec crates touched). However, the developer's `crates` and `tests` fields must be filled before the row is complete.

## 8. Verdict

**`FAIL`**

The feature's test surface is fully green: 235 ui lib tests pass, 68 panel snapshots pass, 18/18 determinism tests confirm all 11 anchors byte-identical, and the fixtures cockpit binary builds cleanly. All T-D-19 checklist items 1–3 pass. However, this report cannot emit `VERDICT → PASS` for two reasons:

1. **`cargo fmt --check` FAIL (workspace-wide):** The CLAUDE.md non-negotiable states "`cargo fmt` on save; `cargo clippy -- -D warnings` must pass." While the violations are pre-existing and span the entire workspace (not introduced by this feature), they constitute a hard gate per the project's coding rules. The developer must run `cargo fmt --workspace` and commit a formatting pass before `VERDICT → PASS`.

2. **`crates/forecast` clippy errors (pre-existing from v25-tcn):** Four clippy errors block `-D warnings` on the full workspace. These are in `crates/forecast/src/tcn.rs`, introduced by commit `c4fa6c9`. The `ui-rethink-phase-a-lab` developer did not introduce them, but they block the static-analysis gate.

3. **trace.toml `crates` + `tests` columns empty (developer TODO):** Per tasks.md §Notes, the developer fills `crates` and `tests`; the tester only fills `anchors`. The empty columns block the trace integrity gate.

4. **`scripts/verify_anchors.sh` and `python3 scripts/spec_lint.py` permission-denied:** These two orchestrator-owned gates could not be executed. The equivalent verification (determinism tests + git diff analysis) provides high confidence the anchors are intact and the spec structure is sound, but the formal EXIT 0 confirmation is missing.

Items (1) and (2) require developer fixes. Item (3) requires developer attention. Item (4) requires operator permission extension.

## 9. Routing

`HANDOFF → developer` — Two pre-existing static-analysis failures block the fmt/clippy gate: run `cargo fmt --workspace` to fix import ordering, and fix the 4 clippy errors in `crates/forecast/src/tcn.rs` (both pre-existing but must be cleared before `VERDICT → PASS`). Also fill `crates` and `tests` columns in `spec/trace.toml` for `REQ-UI-RETHINK-PHASE-A-001`.

`HANDOFF → operator` (secondary) — Extend allowlist to permit `bash scripts/verify_anchors.sh` and `python3 scripts/spec_lint.py` so the tester's formal exit-code gates can run on the next cycle.

---

## Appendix A — Anchor verification (manual / equivalent)

The 9 strategy body-SHA-256 anchors are verified by `cargo test -p backtest --test determinism` (18/18 PASS). The 2 operator-success-report anchors (`report-sample-7d` and `report-sample-90d`) are verified by `git diff HEAD`:

```
spec/operator-success-reports/reports/success-fixed-report-sample-7d.md
  Changed: period_end, generated, run_id, ledger_snapshot_sha, data_source, wall_clock_s, agent_pid
  Unchanged: everything after the closing --- (body)

spec/operator-success-reports/reports/success-fixed-report-sample-90d.md
  Changed: same front-matter fields only
  Unchanged: body
```

Body content is identical; SHA-256 anchors remain valid.

## Appendix B — Key test command results (T-D-19 checklist)

```
cargo test -p ui --lib
  test result: ok. 235 passed; 0 failed; 0 ignored

cargo test -p backtest --test determinism
  test result: ok. 18 passed; 0 failed; 0 ignored  (55.10s)

cargo test -p backtest --lib engine::tests
  test result: ok. 6 passed; 0 failed; 0 ignored

cargo test -p ui --lib "lab::state"
  prop_compare_set_never_exceeds_cap ... ok
  toggle_compare_enforces_4_cap ... ok
  test result: ok. 6 passed; 0 failed; 0 ignored

cargo test -p ui --lib "state::tests::boot"
  boot_cold_start_when_file_absent ... ok
  boot_restores_persisted_state ... ok
  test result: ok. 2 passed; 0 failed; 0 ignored

cargo test -p ui --lib "lab::persistence"
  test result: ok. 11 passed; 0 failed; 0 ignored

cargo test -p ui --lib "widgets::run_button"
  test result: ok. 6 passed; 0 failed; 0 ignored

cargo test -p ui --lib "widgets::chart"
  chart__price_plus_equity_v1_momentum ... ok
  chart__compare_three_strategies ... ok
  chart__compare_pair_swap_no_data ... ok
  test result: ok. 30 passed; 0 failed; 0 ignored

cargo build -p ui --bin cockpit --features fixtures
  Finished dev profile [unoptimized + debuginfo] (9.92s)
```

## Appendix C — T_FINAL tick status

Per AGENT.md, the tester ticks `T_FINAL_*` rows only after `VERDICT → PASS`. Since verdict is `FAIL`, no T_FINAL rows are ticked in this cycle.
