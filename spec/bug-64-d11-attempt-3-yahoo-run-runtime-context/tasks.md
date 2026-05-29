---
slug: bug-64-d11-attempt-3-yahoo-run-runtime-context
status: dev-done
owner: developer
updated: 2026-05-29
---

# Tasks — Bug #64 D.1.1 attempt-3 Yahoo+Run runtime context

## M-T1 — Architect (DONE 2026-05-29)

Architect validation at commit `4473bd2` locked the 8 design
clauses + ADR-0050 obligation. Analyst validation at commit
`ccf39b9` confirmed Q-refresh + recommends one-ship Q1=(a).
All operator-decide Qs closed (Q1..Q5 + A-Q1..A-Q3). No further
M-T1 work required.

- [x] T-M-T1.1 — D-R1.1..D-R2.3 + D-R1.4 + D-Tr.1 locked
  (see `bug-64-arch-validation-2026-05-29.md § 3`).
- [x] T-M-T1.2 — ADR-0050 design (D1, D2, D3) drafted
  (see `bug-64-arch-validation-2026-05-29.md § 5`).
- [x] T-M-T1.3 — Scope decision: Q1=(a) one-ship REAFFIRMED.

## M-DEV — Developer

### Tier 1 — R1 fix (rt_handle context)

- [x] **T-BUG64-D1**: Add `let _guard = rt_handle.enter();`
  at top of the iced::Task::perform async closure in
  `spawn_lab_run` BEFORE the `tokio::time::interval(250ms)`
  construction. File: `crates/ui/src/lab/runner.rs:~744`
  (line may shift slightly). Verify `cargo test -p ui --test
  spawn_lab_run_yahoo_harness --no-default-features --features
  live` still 3/3 PASS.
  **DONE**: `crates/ui/src/lab/runner.rs:752-760` — `let mut ticker = { let _guard = rt.enter(); tokio::time::interval(...) };`.
  Test: `cargo test -p ui --test spawn_lab_run_yahoo_harness --no-default-features --features live` → `3 passed; 0 failed`.

- [x] **T-BUG64-D2** (D-R1.2): Grep audit. Run:
  ```
  grep -rn "tokio::time::\|tokio::spawn\|tokio::select" crates/ui/src/
  ```
  For each match, check the surrounding scope:
  - If inside an `iced::Task::perform` async closure → MUST have
    `rt_handle.enter()` guard. Add if missing.
  - If inside a Subscription/Recipe `stream_impl` → already
    guarded by the recipe's own `enter()` (per ServerTimeRecipe
    precedent at cockpit_live.rs:104-126). Verify.
  - If neither (e.g. inside main() or test code) → skip.
  Document each audited site in feature.md § Implementation.
  **DONE**: Audit complete. runner.rs:744 (now ~757) was the only
  unguarded site — fixed by D1. live.rs:786/831 both have
  `rt_handle.enter()` guards. training_subscription.rs:104 has guard.
  cockpit_live.rs:489/501 are inside rt.block_on (reactor available).
  fetch_with_backoff tokio::time::timeout/sleep: architect confirmed
  preload IO works (reqwest uses internal tokio spawning). No additional
  fixes needed.
  Test: `cargo test -p ui --test spawn_lab_run_yahoo_harness --no-default-features --features live` → `3 passed; 0 failed`.

- [x] **T-BUG64-D3** (D-R1.3): Author e2e test at
  `crates/ui/tests/lab_runner_ticker_e2e.rs`. Test fires a
  Run with a bounded 1 s preload window (use a slow-fake
  YahooBarSource that sleeps 1 s). Assert the ticker
  channel received ≥ 3 progress Messages with monotonically
  increasing `elapsed_ms` values. Cmd: `cargo test -p ui
  --test lab_runner_ticker_e2e --no-default-features --features
  live`.
  **DONE**: `crates/ui/tests/lab_runner_ticker_e2e.rs:79` — `ticker_fires_at_least_3_times_in_1s_window`.
  Test: `cargo test -p ui --test lab_runner_ticker_e2e --no-default-features --features live` → `1 passed; 0 failed; finished in 1.00s`.

- [x] **T-BUG64-D4** (D-R1.4): Add `tokio::task::yield_now().await;`
  at the top of the preload select! loop body. ~3 LoC defense-
  in-depth per architect A-Q1=YES.
  **DONE**: `crates/ui/src/lab/runner.rs:807` — `tokio::task::yield_now().await;` at top of preload loop.
  Test: `cargo test -p ui --test lab_runner_ticker_e2e --no-default-features --features live` → `1 passed; 0 failed`.

### Tier 2 — R2 fix (cancellation)

- [x] **T-BUG64-D5** (D-R2.1): Adopt
  `tokio_util::sync::CancellationToken` in
  `crates/backtest/src/cancel.rs`. Add dep:
  ```toml
  # crates/backtest/Cargo.toml
  tokio-util = { version = "0.7", default-features = false, features = ["rt"] }
  ```
  Verify with the workspace library-compat checklist. Either
  swap RunCancelReceiver internals to wrap CancellationToken
  OR add a `notified() -> impl Future` method that bridges via
  `tokio::sync::Notify`. Architect-recommended: primitive swap.
  Test cmd: `cargo test -p backtest --lib -- cancel`.
  **DONE**: `crates/backtest/src/cancel.rs:1-162` — full primitive swap to CancellationToken. `crates/backtest/Cargo.toml` — `tokio-util = { workspace = true }`.
  Test: `cargo test -p backtest --lib -- cancel` → `7 passed; 0 failed`.

- [x] **T-BUG64-D6** (D-R2.2): Add third arm to the existing
  `tokio::select!` at `crates/ui/src/lab/runner.rs:705-828`
  preload loop:
  ```rust
  cancelled = cancel.cancelled() => {
      return Err(SmolStr::new("operator cancelled during preload"));
  }
  ```
  (or equivalent shape per architect D-R2.2). Verify
  `cargo test -p ui --test lab_stop_button_gating
  --no-default-features --features live` still 3/3 PASS.
  **DONE**: `crates/ui/src/lab/runner.rs:814-828` — `_ = cancel.cancelled() => { ... return Err(SmolStr::new("operator cancelled during preload")); }`.
  Test: `cargo test -p ui --test lab_stop_button_gating --no-default-features --features live` → `3 passed; 0 failed`.

- [x] **T-BUG64-D7** (D-R2.3): Author e2e test at
  `crates/ui/tests/lab_runner_cancel_e2e.rs`. Spawn a Run, send
  Stop within 100 ms of start, assert the run exits within
  500 ms total wall-clock with `Err(...)` containing "cancelled".
  Cmd: `cargo test -p ui --test lab_runner_cancel_e2e
  --no-default-features --features live`.
  **DONE**: `crates/ui/tests/lab_runner_cancel_e2e.rs:72` — `stop_during_preload_exits_within_500ms` + `cancel_before_preload_start_is_instant`.
  Test: `cargo test -p ui --test lab_runner_cancel_e2e --no-default-features --features live` → `2 passed; 0 failed; finished in 0.10s`.

### Tier 3 — ADR-0050 atomic-register

- [x] **T-BUG64-D8**: Author
  `spec/architecture/adr/0050-iced-tokio-runtime-context.md`
  per architect § 5:
  - D1: rt_handle.enter() invariant
  - D2: tokio_util::sync::CancellationToken canonical primitive
  - D3: timer-fired-in-bounded-window test contract
  - Changelog row: "2026-05-29 (architect+developer): codified
    on 3rd recurrence per twice-bitten threshold; see
    bug-64-arch-validation-2026-05-29.md § 5."
  **DONE**: `spec/architecture/adr/0050-iced-tokio-runtime-context-and-cancellation.md` authored with all 3 D-clauses + Changelog.
  Test: file exists and contains D1/D2/D3 — `ls spec/architecture/adr/ | grep 0050` confirms presence.

- [x] **T-BUG64-D9** (atomic-register): In the SAME commit:
  - Append a row to `spec/architecture/adr/README.md` table.
  - Bump `spec/architecture/adr/README.md` frontmatter
    `updated:` field to 2026-05-29.
  - Append amendment row to `spec/architecture/adr/0048-lab-
    recipe-test-harness.md § Changelog` (ride-along — the new
    e2e tests extend Surface 1 contract).
  **DONE**: `spec/architecture/adr/README.md` — ADR-0050 row appended to registry table; frontmatter `updated:` bumped to 2026-05-29. `spec/architecture/adr/0048-lab-recipe-test-harness.md` — Changelog amended with Bug #64 attempt-3 ride-along row.
  Test: ADR-0050 file exists, README table has row 0050, 0048 has new Changelog entry — all verifiable in SAME commit.

### Tier 4 — Spec hygiene

- [x] **T-BUG64-D10** (D-Tr.1): Add `REQ-BUG-64-D-11-ATTEMPT-3-001`
  row to `spec/trace.toml`. Columns: `arch` (point at architect
  validation + this feature.md § Design + ADR-0050), `crates`
  (point at modified files), `tests` (point at new e2e tests +
  regression-verified suites), `anchors = []` (zero anchor
  delta), `state = "dev-done"` at completion.
  **DONE**: `spec/trace.toml:2351` — `[[req]]` row `REQ-BUG-64-D-11-ATTEMPT-3-001` with all columns wired. `state = "dev-done"`.
  Test: `grep REQ-BUG-64-D-11-ATTEMPT-3-001 spec/trace.toml` — row present.

- [x] **T-BUG64-D11**: Update `spec/dev-notes/operator-side-
  pending-ledger.md` Bug #64 row: move from FAILED → fix-in-
  flight with link to this feature folder.
  **DONE**: `spec/dev-notes/operator-side-pending-ledger.md:32` — Status updated FAILED → fix-in-flight with link to feature folder + Changelog entry.
  Test: `grep "fix-in-flight" spec/dev-notes/operator-side-pending-ledger.md` — entry present.

- [x] **T-BUG64-D12**: Standard gates:
  - `cargo fmt --all --check` → zero diff
  - `cargo clippy -p ui -p backtest --tests -- -D warnings` →
    zero new errors
  - `bash scripts/verify_anchors.sh` → 84/84 PASS (R-NR.1)
  - All R-NR.2 regression suites PASS (see feature.md)
  **DONE**:
  - `cargo fmt --all --check` → EXIT 0 (zero diff).
  - `cargo clippy -p ui -p backtest --tests --no-default-features --features live -- -D warnings` → zero NEW errors from touched files (cancel.rs, runner.rs new code, lab_runner_*.rs). Pre-existing errors in other files are carry-forward.
  - `bash scripts/verify_anchors.sh` → `ANCHORS PASS (84 / 84)`.
  - R-NR.2: spawn_lab_run_yahoo_harness 3/3, lab_stop_button_gating 3/3, training_log_recipe_harness 3/3.
  - New e2e: lab_runner_ticker_e2e 1/1, lab_runner_cancel_e2e 2/2.
  - Release build: `cargo build --release -p ui --bin cockpit_live --features live,yahoo` → Finished.

## M-FINAL — Tester

- [ ] T-BUG64-FINAL.1 — Re-run all 12 dev tasks in tester
  environment.
- [ ] T-BUG64-FINAL.2 — Verify the 2 NEW e2e tests fire ≥ 3
  ticker Messages + ≤ 500 ms cancel exit.
- [ ] T-BUG64-FINAL.3 — Regression: 3/3 spawn_lab_run_yahoo_
  harness + 3/3 lab_stop_button_gating + 3/3 Wave A
  training_log_recipe_harness.
- [ ] T-BUG64-FINAL.4 — verify_anchors.sh 84/84 PASS.
- [ ] T-BUG64-FINAL.5 — ADR-0050 atomic-register verified
  (file + README row + README frontmatter all in one commit).
- [ ] T-BUG64-FINAL.6 — Write `spec/bug-64-d11-attempt-3-
  yahoo-run-runtime-context/reports/test-<ts>-v0.1.0.md` per
  test-report template.

## M-PRESENT — Presenter (after operator re-verify PASS)

- [ ] T-BUG64-PRESENT.1 — Assemble v0.1.0 presentation deck.
- [ ] T-BUG64-PRESENT.2 — Emit operator-recipe in deck for
  re-verify (cold-cache Yahoo run + Stop-during-fetch
  manual test).
- [ ] T-BUG64-PRESENT.3 — Approval block.
