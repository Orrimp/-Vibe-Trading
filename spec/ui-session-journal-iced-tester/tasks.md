---
slug: ui-session-journal-iced-tester
status: shipped
owner: shipped
updated: 2026-05-16
---

# Tasks — Session journal `iced_tester` adapter v0.1

> ## Ship status (2026-05-16)
>
> - **T01** ✓ — `record-tests` feature in `crates/ui/Cargo.toml`
> - **T02** ✗→deleted — CLI flag removed; iced auto-attaches via
>   `tester` feature (`iced::Application::run()` does the wrap
>   unconditionally when `cfg(feature = "tester")`)
> - **T03** ✓ (simplified) — no manual `iced_tester::attach()` call;
>   one `tracing::info!` line on the `record-tests` build path
> - **T04** ✓ — `crates/ui/tests/journal_replay.rs` walks
>   `recorded-sessions/` via `iced_test::run`; passes vacuously on
>   empty dir
> - **T05** ⊘ deferred — first recording requires desktop interaction;
>   operator records post-ship
> - **T06** ✓ — V1, V4, V5, V6, V7, V8 all green; 1223 workspace
>   tests pass; clippy + fmt clean
>
> Net effort: ~3 hours actual (vs ~6.5h estimate). The Q-ARCH-1
> resolution (iced auto-attaches) collapsed T02 + T03 from 1.5h
> + 2h to ~30 min.

> Effort budget: **~1 dev-day** total per
> [`iced-014-feature-analysis-2026-05-15.md §5`](../dev-notes/iced-014-feature-analysis-2026-05-15.md#what-this-means-for-ui-session-journal).
> Task estimates below sum to **6.0 hours** (one focused dev day).
>
> Honest-tick discipline (per
> [`AGENT.md ## Process discipline`](../../AGENT.md#process-discipline-lessons-from-v0--v15a)):
> developer MUST NOT tick `[x]` without citing
> (a) file:line of change, (b) test command, (c) test-output line.

## M0 — Architect design pass (0.5h)

Two `Q-ARCH-N` resolutions required before T03/T04 lands.

- [ ] **T-M0-A** *(architect)* — Resolve **Q-ARCH-1**
  ([feature.md ## Open questions for architect](feature.md#open-questions-for-architect)):
  exact composition of `iced::application(...)` builder output with
  `iced_tester::attach()`. Architect reads
  [iced::Application docs](https://docs.rs/iced/0.14.0/iced/application/index.html)
  + [iced_tester::attach source](https://github.com/iced-rs/iced/blob/0.14.0/tester/src/lib.rs)
  and prescribes one of:
  - **Path A** — `attach(builder).run()` (wrap builder output)
  - **Path B** — `attach(boot, update, view).title(...)...run()` (attach is entry-point)
  Decision lands in `spec/ui-session-journal-iced-tester/design.md`
  (NEW addendum, like `ui-gallery-bin/design.md`). 15 min.

- [ ] **T-M0-B** *(architect)* — Resolve **Q-ARCH-2**: exact signature
  of `iced_test::run`. Read
  [iced_test docs](https://docs.rs/iced_test/0.14.0/iced_test/) +
  source. Prescribe the `journal_replay.rs` call shape. 15 min.

## M1 — Cargo feature + CLI flag (1.5h)

Code lands in two files. No new external crate deps.

- [ ] **T01** — Add `record-tests` feature stanza to
  `crates/ui/Cargo.toml` `[features]`:
  ```toml
  record-tests = ["iced/tester", "iced/selector", "iced/strict-assertions"]
  ```
  Verify with: `cargo build -p ui --features live,record-tests`
  (should succeed) AND `cargo build -p ui --features live` (should
  NOT pull `iced_tester` — confirm with `cargo tree -p ui --features live`).
  30 min.

- [ ] **T02** — Add `--record-tests` boolean CLI flag to `Args` in
  [`crates/ui/src/bin/cockpit_live.rs`](../../crates/ui/src/bin/cockpit_live.rs)
  (line ~149). Gate with `#[cfg(feature = "record-tests")]` so the
  flag is invisible in `--features live` builds. Verify:
  ```bash
  # With feature: --help shows --record-tests
  cargo run -p ui --features live,record-tests --bin cockpit_live -- --help
  # Without feature: --help does NOT show --record-tests
  cargo run -p ui --features live --bin cockpit_live -- --help
  ```
  1 hour.

## M2 — Recorder wiring (2.0h)

The actual `iced_tester::attach()` integration. Per Q-ARCH-1 (T-M0-A).

- [ ] **T03** — Wire `iced_tester::attach(application).run()` (or
  Q-ARCH-1's prescribed variant) around the
  [`iced::application(...)`](../../crates/ui/src/bin/cockpit_live.rs)
  call at cockpit_live.rs:458. Gate with `#[cfg(feature =
  "record-tests")]` + `if args.record_tests { ... }`. Add a
  `tracing::info!("iced_tester recorder attached")` log line on the
  enabled branch. Manual smoke:
  ```bash
  cargo run -p ui --features live,record-tests --bin cockpit_live -- --record-tests
  ```
  Operator confirms the recorder overlay is visible. 2 hours
  (includes T-M0-A fallback contingency if Path A fails to compile).

## M3 — Replay harness (2.0h)

The `journal_replay.rs` walker + first recorded session.

- [ ] **T04** — Author
  [`crates/ui/tests/journal_replay.rs`](../../crates/ui/tests/journal_replay.rs)
  per the feature.md "Replay harness" section. Uses
  `iced_test::ice::Ice::parse` + Q-ARCH-2's prescribed
  `iced_test::run` signature.
  ```bash
  mkdir -p crates/ui/tests/recorded-sessions
  touch crates/ui/tests/recorded-sessions/.gitkeep
  cargo test -p ui --test journal_replay
  # Should exit 0 with "replayed 0 recorded session(s)" on empty dir.
  ```
  1.5 hours.

- [ ] **T05** — Record one canonical session as the v0.1 reference.
  Operator records (T03 must be done): boots `cockpit_live` with
  `--record-tests`, clicks around for ~30s, exports `.ice` via the
  overlay's file dialog to
  `crates/ui/tests/recorded-sessions/v0p1-smoke.ice`. Developer
  hand-edits the `.ice` to add at least one `Expect::*` assertion
  (per R-RT-2 mitigation). Verify:
  ```bash
  cargo test -p ui --test journal_replay
  # Should now report "replayed 1 recorded session(s)" and exit 0.
  ```
  If T05 produces no recording (operator unavailable or recorder
  blocks): skip the file, ship empty directory, document in feature.md
  changelog. Test still passes (V4 accepts 0 sessions). 0.5 hours.

## M4 — Verification gate (0.5h)

Compile + lint + test gates before HANDOFF → tester.

- [ ] **T06** — Run the full V-item verification block from
  [feature.md ## Acceptance / verification](feature.md#acceptance--verification-v-items):
  - V1: `cargo build -p ui --features live,record-tests --bin cockpit_live` → succeeds
  - V4: `cargo test -p ui --test journal_replay` → exits 0
  - V5: `cargo build -p ui --features live --bin cockpit_live` → succeeds, no iced_tester linkage
  - V6: `cargo test --workspace` → 1222+ pass, 0 fail (no regression)
  - V7: `cargo clippy -p ui --no-deps` → no new warnings
  - V8: `cargo fmt --check` → clean

  Record each command's exit code + last 5 lines in a
  `spec/ui-session-journal-iced-tester/reports/test-run-<timestamp>-ui-session-journal-iced-tester.md`
  via `spec-update`. 30 min.

## M_FINAL_TEST_RUN — tester pass

- [ ] **T_FINAL** *(tester)* — Tester runs the full
  `rust-build` + `rust-validate` + `rust-test` skill suite. Anchors
  are not at risk (no strategy/audit/exec/backtest code touched);
  `verify-anchors` is the formal gate regardless. VERDICT → PASS
  required for handoff to presenter.

## M_FINAL_PRESENT — presenter pass

- [ ] **T_PRES** *(presenter)* — Author
  `spec/ui-session-journal-iced-tester/presentations/ui-session-journal-iced-tester-2026-MM-DD.md`
  via the `present-results` skill. Operator approval gates ship.

## Effort summary

| Milestone | Hours |
|---|---|
| M0 — architect | 0.5 |
| M1 — feature + CLI | 1.5 |
| M2 — recorder wiring | 2.0 |
| M3 — replay harness | 2.0 |
| M4 — verification | 0.5 |
| **Total** | **6.5 hours (~1 dev-day)** |

Plus tester + presenter passes (~1 hour combined).

## Status notes

- 2026-05-15 (orchestrator): tasks.md authored. Status: queued. No
  spawn trigger pulled. Next action awaits operator promotion via the
  normal pipeline.
