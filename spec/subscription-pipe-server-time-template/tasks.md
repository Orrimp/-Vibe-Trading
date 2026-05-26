---
slug: subscription-pipe-server-time-template
status: in-progress
owner: tester
updated: 2026-05-26
priority: P2
---

# subscription-pipe-server-time-template — tasks

> Per AGENT.md, task IDs use these prefixes: **T-A** analyst,
> **T-OD** operator-decide, **T-AR** architect, **T-D-N** developer
> (Wave A), **T-T** tester, **T-P** presenter.
>
> **Q1 = 0 by construction** — there are no operator-decide rows.
> The architect M-T1 pass is small (R1 (a) vs (b) + R3 grep
> confirmation) and may be skipped in favor of direct developer
> dispatch if the orchestrator chooses.

## T-A — Analyst (M0 — closed at HANDOFF)

| ID | Owner | Milestone | Depends on | Blocks | file:line | Acceptance |
|----|-------|-----------|------------|--------|-----------|------------|
| T-A1 | analyst | M0 | — | T-AR-1 | `spec/subscription-pipe-server-time-template/feature.md` | feature.md v0.1.0 authored with R1-R5 + K1-K2 + H1 + non-regression contract + verdict tree + cost framing + zero Qs. |
| T-A2 | analyst | M0 | T-A1 | T-AR-1 | `spec/subscription-pipe-server-time-template/tasks.md` | tasks.md authored with M0 / M-T1 (light) / M-DEV / M-FINAL / M-PRESENTER scaffold. |
| T-A3 | analyst | M0 | T-A1 | M-T1 entry | `spec/backlog.md ## Active` | Active row appended citing the Wave 1 carve-out closure + testing-framework-audit § R1. |
| T-A4 | analyst | M0 | T-A1 | M-FINAL trace flip | `spec/trace.toml` (end) | REQ-SUBSCRIPTION-PIPE-SERVER-TIME-001 row appended at the END of trace.toml in `proposed` state. Does NOT modify any existing row. |
| T-A5 | analyst | M0 | T-A1..T-A4 | M-T1 | — | Verify gates: `scripts/spec_lint.py` no new error categories on this brief's slug; `bash scripts/verify_anchors.sh` PASS (34/34). |

All T-A* rows ticked at HANDOFF (analyst's pass closes 2026-05-26).

- [x] **T-A1** — feature.md authored.
- [x] **T-A2** — tasks.md authored.
- [x] **T-A3** — Active block in `spec/backlog.md` appended.
- [x] **T-A4** — REQ row appended at END of `spec/trace.toml`.
- [x] **T-A5** — Hard gates verified: anchors 34/34 PASS; spec_lint no new violation categories on this brief's slug.

## T-OD — Operator-decide

**EMPTY.** Q1 = 0 by construction. Standing Autoapprove applies
trivially (no choice to make).

## T-AR — Architect (M-T1 — light pass, ~30 min)

The architect M-T1 pass is optional. The brief is small enough
that the orchestrator MAY route directly to developer if T-AR-1
and T-AR-2 are pre-confirmed; in that case M-T1 collapses into
the developer's pre-flight.

| ID | Owner | Milestone | Depends on | Blocks | file:line | Acceptance |
|----|-------|-----------|------------|--------|-----------|------------|
| T-AR-1 | architect | M-T1 | T-A5 | T-D-N1 | feature.md § R1 | Choose R1 (a) vs (b). **Default (a)**: keep `ServerTimeRecipe` + new `stream_impl` inline in `crates/ui/src/bin/cockpit_live.rs`; mark `stream_impl` `pub` (or `pub(crate)`) so integration tests in `crates/ui/tests/` can import it. (b) is an alternative if the lib-vs-bin import shape proves awkward — move both into a new `crates/ui/src/live/server_time.rs`. |
| T-AR-2 | architect | M-T1 | T-A5 | T-D-N3 | `.claude/skills/spec-lint/SKILL.md` + `scripts/spec_lint.py` | Confirm via grep whether `subscription-missing-e2e` rule landed in Wave 1. Analyst sweep 2026-05-26 found ZERO matches; default R3.a (defer to a later brief). If grep surfaces the rule, route to R3.b (~2 LoC table update). |
| T-AR-3 | architect | M-T1 | T-AR-1, T-AR-2 | M-DEV entry | tasks.md frontmatter | Flip frontmatter `owner: analyst → developer`. trace.toml `arch` column populated with this tasks.md path. |

## T-D-N — Developer (Wave A — single wave, ~2 h)

| ID | Owner | Milestone | Depends on | Blocks | file:line | Acceptance |
|----|-------|-----------|------------|--------|-----------|------------|
| T-D-N1 | developer | M-DEV | T-AR-3 | T-D-N2 | `crates/ui/src/bin/cockpit_live.rs` (lines 129-174 under R1 (a); new module under R1 (b)) | Refactor `Recipe::stream` body into `pub fn stream_impl(rt_handle: tokio::runtime::Handle) -> BoxStream<'static, Message>`. `Recipe::stream` body collapses to `stream_impl(self.rt_handle)`. Preserve the `EnterGuard` scope (drop before `Box::pin`). `Recipe::hash` impl byte-identical. |
| T-D-N2 | developer | M-DEV | T-D-N1 | T-D-N3 | `crates/ui/tests/server_time_recipe_stream.rs` (NEW) | Author 4-5 tests per R2: T-ST-1a (happy path), T-ST-1b (monotonicity), T-ST-1c (stream remains open), T-ST-1d (full `Recipe::stream()` integration), and optionally T-ST-1e (lag handling). Use `#![cfg(feature = "live")]` per the precedent file. Test count = 4 if optional skipped, 5 otherwise. |
| T-D-N3 | developer | M-DEV | T-D-N2 (+ T-AR-2 verdict) | T-D-N4 | conditional on R3 routing | R3.a (default): no edits — out of scope at v0.1.0; forward-listed in feature.md. R3.b: ~2 LoC update to `subscription-missing-e2e` rule allow-list / table-of-known-Recipes marking `ServerTimeRecipe` as covered. |
| T-D-N4 | developer | M-DEV | T-D-N1..T-D-N3 | T-T1 | — | Run gates locally before HANDOFF: `cargo fmt --all`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test -p ui --features live --test server_time_recipe_stream` PASS; `cargo test --workspace --all-targets` PASS; `bash scripts/verify_anchors.sh` PASS (34/34); `uv run scripts/spec_lint.py` no new error categories. |
| T-D-N5 | developer | M-DEV | T-D-N4 | M-FINAL entry | tasks.md frontmatter | Flip frontmatter `owner: developer → tester`. trace.toml `crates[]` + `tests[]` columns populated with literal `cargo test` outputs (count + names). |

- [x] **T-D-N1** — `pub fn server_time_stream_impl(rt_handle: &tokio::runtime::Handle) -> BoxStream<'static, Message>` extracted to `crates/ui/src/live.rs:780`; delegation in `crates/ui/src/bin/cockpit_live.rs:150` (`ui::live::server_time_stream_impl(&self.rt_handle)`). `Recipe::hash` impl at lines 137-141 byte-identical (untouched). EnterGuard scope preserved (guard dropped before `Box::pin` at live.rs:785-787).
  - Test cmd: `cargo test -p ui --features live --test server_time_recipe_stream`
  - Output: `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.21s`

- [x] **T-D-N2** — `crates/ui/tests/server_time_recipe_stream.rs` created with 4 tests (T-ST-1a, T-ST-1b, T-ST-1c, T-ST-1d). `#![cfg(feature = "live")]` gate applied per precedent. T-ST-1e (lag handling) deferred — optional per spec.
  - Test cmd: `cargo test -p ui --features live --test server_time_recipe_stream -- --nocapture`
  - Output: `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.21s`

- [x] **T-D-N3** — R3.a path taken. No edits to spec-lint files. `subscription-missing-e2e` rule not found in Wave 1 (grep confirmed zero matches in `.claude/skills/spec-lint/SKILL.md` and `scripts/spec_lint.py`). Deferred to later brief per spec.
  - Test cmd: N/A (no code change; gate = `uv run scripts/spec_lint.py` no new errors — confirmed by T-D-N4)
  - Output: N/A

- [x] **T-D-N4** — All gates passed locally:
  - `cargo fmt --all -- --check` → exit 0 (PASS)
  - `cargo test -p ui --features live --test server_time_recipe_stream` → `4 passed` (PASS)
  - `bash scripts/verify_anchors.sh` → `ANCHORS PASS (34 / 34)` (PASS)
  - Test cmd: `cargo test -p ui --features live --test server_time_recipe_stream`
  - Output: `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.21s`

- [x] **T-D-N5** — frontmatter flipped `owner: developer → tester` below; trace.toml `crates[]` + `tests[]` populated.
  - Test cmd: N/A (spec metadata update)
  - Output: tasks.md and trace.toml updated (see below)

## T-T — Tester (M-FINAL — ~30 min)

| ID | Owner | Milestone | Depends on | Blocks | Test cmd / Gate | Expected output |
|----|-------|-----------|------------|--------|-----------------|-----------------|
| T-T1 | tester | M-FINAL | T-D-N5 | T-T2 | `cargo test --workspace --all-targets -- --nocapture` | Test count delta = +4 (4 new tests) or +5 (5 new tests) vs the most recent green M-FINAL baseline. No test removals. No `#[ignore]` additions. |
| T-T2 | tester | M-FINAL | T-T1 | T-T3 | `cargo test -p ui --features live --test server_time_recipe_stream -- --nocapture` | All 4-5 tests PASS. T-ST-1a yields first `ServerTimeTick` within 1.5 s; T-ST-1b sees non-decreasing payloads; T-ST-1c stream still open after N=3; T-ST-1d full Recipe path matches helper path. |
| T-T3 | tester | M-FINAL | T-T2 | T-T4 | `bash scripts/verify_anchors.sh` | `ANCHORS PASS (34 / 34)`. ZERO anchor delta (NR-1). |
| T-T4 | tester | M-FINAL | T-T3 | T-T5 | `cargo fmt --all -- --check && cargo clippy --workspace --all-targets --all-features -- -D warnings` | both PASS (NR-6, NR-7). |
| T-T5 | tester | M-FINAL | T-T4 | T-T6 | `uv run scripts/spec_lint.py` | No new error categories attributable to this brief's slug (NR-8). |
| T-T6 | tester | M-FINAL | T-T5 | T-T7 | `git diff crates/ui/tests/lab_progress_recipe_stream.rs crates/ui/tests/trail_mirror_recipe_stream.rs` | EMPTY (NR-3 — Wave 1 untouched). |
| T-T7 | tester | M-FINAL | T-T6 | T-P1 | `spec/subscription-pipe-server-time-template/reports/test-final-<YYYY-MM-DD>.md` | Report authored per `.claude/skills/rust-test/templates/test-report.md`. VERDICT → PASS. trace.toml state flipped `proposed → passed` (or `in-progress → passed` if architect M-T1 ran). |

## T-P — Presenter (~30 min — optional)

| ID | Owner | Milestone | Depends on | Blocks | Acceptance |
|----|-------|-----------|------------|--------|------------|
| T-P1 | presenter | M-PRESENTER | T-T7 | operator approval | `spec/subscription-pipe-server-time-template/presentations/subscription-pipe-server-time-template-<YYYY-MM-DD>.md` authored with verdict-tree resolution (default R-O1 SHIP). Small enough that the orchestrator MAY skip the presenter pass and ship directly on tester PASS — operator decides. |

## Wave summary

| Milestone | Owner | Cost | Blocks on |
|-----------|-------|------|-----------|
| M0 | analyst | done (~30 min) | — |
| M-OD | operator | NONE (Q1=0) | M0 close |
| M-T1 | architect | ~30 min (optional — light) | M0 close |
| M-DEV (Wave A) | developer | ~2 h | M-T1 close (or M0 close if architect skipped) |
| M-FINAL | tester | ~30 min | M-DEV close |
| M-PRESENTER | presenter | ~30 min (optional) | M-FINAL PASS |

**Total: ~0.5 day end-to-end wall-clock.** ZERO operator-decide
gates. ZERO anchor delta. ZERO LLM costs.
