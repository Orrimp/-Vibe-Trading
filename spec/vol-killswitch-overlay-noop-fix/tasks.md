---
slug: vol-killswitch-overlay-noop-fix
status: in-progress
owner: analyst
updated: 2026-05-26
priority: P0
---

# vol-killswitch-overlay-noop-fix — tasks

> Per AGENT.md, task IDs use these prefixes: **T-A** analyst,
> **T-OD** operator-decide, **T-AR** architect, **T-D-N** developer
> (Wave A — single wave for this 1-day fix), **T-T** tester,
> **T-P** presenter. Architect refines the developer wave at M-T1.

## T-A — Analyst (M0 — closed at HANDOFF)

| ID | Owner | Milestone | Depends on | Blocks | file:line | Acceptance |
|----|-------|-----------|------------|--------|-----------|------------|
| T-A1 | analyst | M0 | — | T-OD1..T-OD3 | `spec/vol-killswitch-overlay-noop-fix/feature.md` | feature.md v0.1.0 authored with R1-R6 + K1-K6 + H1-H3 + Q1-Q3 + non-regression contract + verdict tree + cost framing. |
| T-A2 | analyst | M0 | T-A1 | T-AR1 | `spec/vol-killswitch-overlay-noop-fix/tasks.md` | tasks.md authored with M0 / M-OD / M-T1 / M-DEV / M-FINAL / M-PRESENTER scaffold. |
| T-A3 | analyst | M0 | T-A1 | M-T1 entry | `spec/backlog.md ## Active` | Active row appended citing Bug #65 + safety framing. |
| T-A4 | analyst | M0 | T-A1 | M-FINAL trace flip | `spec/trace.toml` (end) | REQ-VOL-KILLSWITCH-NOOP-FIX-001 row appended at the END of trace.toml in `proposed` state. Does NOT modify any existing row (parallel architect owns REQ-REFLECTION-TRADER-001 at line 1084). |
| T-A5 | analyst | M0 | T-A1 | bug-log close | `spec/bug-log.md` § #65 | Bug log #65 row updated to `Status: open (analyst brief authored)` + cross-link to `spec/vol-killswitch-overlay-noop-fix/feature.md`. |
| T-A6 | analyst | M0 | T-A1..T-A5 | M-OD | — | Verify gates: `scripts/spec_lint.py` no new errors; `bash scripts/verify_anchors.sh` PASS (34/34). |

All T-A* rows ticked at HANDOFF (this analyst pass).

- [x] **T-A1** — feature.md authored.
- [x] **T-A2** — tasks.md authored.
- [x] **T-A3** — Active block in `spec/backlog.md` appended.
- [x] **T-A4** — REQ row appended at END of `spec/trace.toml`.
- [x] **T-A5** — `spec/bug-log.md` § #65 row updated to `open (analyst brief authored)`.
- [x] **T-A6** — Hard gates verified: spec_lint inherits existing drift only; verify_anchors PASS (34/34).

## T-OD — Operator-decide (Q1..Q3)

Standing Autoapprove from the v3-volatility-forecaster-noop-fix
2026-05-22 precedent applies to the analyst-recommended defaults.
Orchestrator may auto-tick all three before spawning the architect.

| ID | Owner | Milestone | Depends on | Blocks | Default | Acceptance |
|----|-------|-----------|------------|--------|---------|------------|
| T-OD1 | orchestrator | M-OD | T-A6 | T-AR1 | **Q1=(i)** mutate `Signal::kind = Hold` on trigger | Resolution recorded in feature.md § Operator-decide. Standing Autoapprove ticks the default. |
| T-OD2 | orchestrator | M-OD | T-A6 | T-AR1 | **Q2=(a)** zero new anchors at v0.1.0 | Same. |
| T-OD3 | orchestrator | M-OD | T-A6 | T-AR1 | **Q3=(a)** defer trait-shape decision | Same. |
| T-OD4 | orchestrator | M-OD | T-OD1..T-OD3 | M-T1 entry | — | Frontmatter flipped `status: draft → in-progress`, `owner: analyst → architect`. spec/trace.toml state flipped `proposed → in-progress`. |

## T-AR — Architect (M-T1)

Architect locks the fix shape after a 5-minute H1 falsification
probe + a single-wave decomposition.

| ID | Owner | Milestone | Depends on | Blocks | file:line | test cmd | Expected output |
|----|-------|-----------|------------|--------|-----------|----------|-----------------|
| T-AR1 | architect | M-T1 | T-OD4 | T-D-N1 | `crates/strategy/src/vol_killswitch_overlay.rs:169-244` + `crates/strategy/tests/vol_killswitch_overlay_end_to_end.rs:163-180` | (probe) `cargo test -p strategy --test vol_killswitch_overlay_end_to_end trigger_fires_and_equity_diverges -- --ignored --nocapture` | Captures the returned signal vectors with their symbols on the spike bar. If `sig.symbol != bar.symbol` for any returned signal → H1 confirmed; lock Q1=(i) fix shape. If all signals' symbols match bar.symbol → H1 wrong; architect re-routes per R-O3. |
| T-AR2 | architect | M-T1 | T-AR1 | T-D-N1 | `spec/vol-killswitch-overlay-noop-fix/decomp.md` (NEW) | — | decomp.md authored with T-AR1 probe findings + fix shape lock + Wave A decomposition (T-D-N1..T-D-N5) + R6 unit-test shape + forensic-gate FAIL/PASS protocol (mirrors precedent's T-D-N3a/3b). |
| T-AR3 | architect | M-T1 | T-AR2 | T-D-N1 | `spec/architecture/adr/0038-vol-forecast-verdict-shape.md` (read-only audit) | — | Confirms § D6.b applies trivially (zero anchor delta; empty enumeration satisfies the "affected anchors" clause). No ADR amendment required by default; if architect surfaces a need at M-T1, files a § D6.c documentation-link-fix variant. |
| T-AR4 | architect | M-T1 | T-AR2 | T-D-N4 | `crates/strategy/tests/vol_killswitch_overlay_end_to_end.rs:169` + `:237` | (forensic) `cargo test -p strategy --test vol_killswitch_overlay_end_to_end -- --ignored --nocapture` | Pre-fix expected FAIL output captured verbatim into decomp.md (developer reproduces at T-D-N3a). |
| T-AR5 | architect | M-T1 | T-AR2 | M-DEV entry | feature.md § Design (append) | — | § Design block appended with cross-pointer to decomp.md. Frontmatter flips `owner: architect → developer`. trace.toml `arch` column populated. |

## T-D-N — Developer (Wave A — single wave, ~1 day)

Wave A is a single sequential wave per H3 (the fix is single-file,
single-method-body, < 20 LoC). The wave includes the pre-fix
forensic FAIL → post-fix PASS bracket per the precedent's
T-D-N3a/3b protocol.

| ID | Owner | Milestone | Depends on | Blocks | file:line | test cmd | Expected output |
|----|-------|-----------|------------|--------|-----------|----------|-----------------|
| T-D-N1 | developer | M-DEV | T-AR5 | T-D-N2 | `crates/strategy/tests/vol_killswitch_overlay.rs` (NEW row OR new file `crates/strategy/tests/vol_killswitch_overlay_unit.rs`) | `cargo test -p strategy --test vol_killswitch_overlay` (or per architect's pick at T-AR2) | R6 unit test added; pre-fix RED with literal panic capturing the no-op signature (e.g. `'kill_active true but Hold signals == 0 on rebalance basket'`). Architect picks the exact test name + assertion shape at T-AR2. |
| T-D-N2 | developer | M-DEV | T-D-N1 | T-D-N3 | `crates/strategy/src/vol_killswitch_overlay.rs:229-244` | `cargo check -p strategy` | Wire-up fix lands per Q1=(i) — kind-mutation scope widened to cover the basket. ~10-20 LoC change. `Finished 'dev' profile [unoptimized + debuginfo] target(s) in <Ns>` |
| T-D-N3 | developer | M-DEV | T-D-N2 | T-D-N4 | `crates/strategy/tests/vol_killswitch_overlay.rs` (R6 unit test from T-D-N1) | `cargo test -p strategy --test vol_killswitch_overlay` | Post-fix PASS — the R6 unit test that was RED at T-D-N1 turns GREEN. `test result: ok. N passed; 0 failed; 0 ignored; 0 measured; 0 filtered out` |
| T-D-N4 | developer | M-DEV | T-D-N3 | T-D-N5 | `crates/strategy/tests/vol_killswitch_overlay_end_to_end.rs:169` + `:237` | `cargo test -p strategy --test vol_killswitch_overlay_end_to_end` | Remove `#[ignore = "tracked-in: bug-log #65 vol_killswitch_overlay no-op"]` annotations from `trigger_fires_and_equity_diverges` (line 169) + `post_trigger_signals_are_hold` (line 237). Run un-ignored. `test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out` |
| T-D-N5 | developer | M-DEV | T-D-N4 | M-FINAL entry | workspace | `cargo fmt --check && cargo clippy --workspace --features candle,realdata -- -D warnings && cargo test --workspace --features candle,realdata && bash scripts/verify_anchors.sh` | Workspace gate PASS: fmt clean; clippy 0 warnings; all tests PASS; `ANCHORS PASS (34 / 34)`. Developer changelog entry appended to feature.md § Changelog with literal outputs. Frontmatter flips `owner: developer → tester`. trace.toml `crates` / `tests` columns populated. |

## T-T — Tester (M-FINAL)

| ID | Owner | Milestone | Depends on | Blocks | file:line | test cmd | Expected output |
|----|-------|-----------|------------|--------|-----------|----------|-----------------|
| T-T1 | tester | M-FINAL | T-D-N5 | T-T2 | workspace | `cargo fmt --check` | PASS (no output — clean). |
| T-T2 | tester | M-FINAL | T-T1 | T-T3 | workspace | `cargo clippy --workspace --features candle,realdata -- -D warnings` | `Finished 'dev' profile [unoptimized + debuginfo] target(s) in <Ns>` (0 warnings). |
| T-T3 | tester | M-FINAL | T-T2 | T-T4 | workspace | `cargo test --workspace --features candle,realdata` | All workspace tests PASS; workspace fail count delta = -2 ignored (the 2 `#[ignore]`'s removed); zero new test failures. |
| T-T4 | tester | M-FINAL | T-T3 | T-T5 | `crates/strategy/tests/vol_killswitch_overlay_end_to_end.rs` | `cargo test -p strategy --test vol_killswitch_overlay_end_to_end` | `test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`. Confirms R2 + R3 + R4 all PASS; the 2 `#[ignore]` annotations removed cleanly. |
| T-T5 | tester | M-FINAL | T-T4 | T-T6 | `spec/anchors.toml` | `bash scripts/verify_anchors.sh` | `ANCHORS PASS (34 / 34)` — 34 anchors byte-identical (zero delta; vol_killswitch does not appear in any anchored scenario). Negative invariant confirmed. |
| T-T6 | tester | M-FINAL | T-T5 | M-PRESENTER entry | `spec/vol-killswitch-overlay-noop-fix/reports/test-final-<YYYY-MM-DD>.md` (NEW) | — | Test report written following `.claude/skills/rust-test/templates/test-report.md`. Verdict: **PASS**. Carries all cargo outputs + verify_anchors PASS + the 3 e2e test results + non-regression contract attestation. Frontmatter flips `owner: tester → presenter`. trace.toml `anchors` column populated ("34/34 PASS"). |

## T-P — Presenter (M-PRESENTER)

| ID | Owner | Milestone | Depends on | Blocks | file:line | Acceptance |
|----|-------|-----------|------------|--------|-----------|------------|
| T-P1 | presenter | M-PRESENTER | T-T6 | M-OPERATOR | `spec/vol-killswitch-overlay-noop-fix/presentations/vol-killswitch-overlay-noop-fix-<YYYY-MM-DD>.md` (NEW) | Operator-approval deck assembled. Carries: (1) Bug #65 framing + safety severity; (2) the 3-cell verdict tree from feature.md § Pre-drawn verdict routing tree; (3) the 1 PASS → 3 PASS test count delta; (4) 34/34 anchors byte-identical attestation; (5) recommend **R-O1 → SHIP** path under standing Autoapprove. HANDOFF → orchestrator → operator-approve. |

## Watch recipes (per MEMORY.md)

If any cargo step at M-DEV / M-FINAL runs > 2 min, the developer
emits a copy-pasteable `watch -n 2 '<probe>'` block in the wave-
status update so the orchestrator can stream progress without
polling. Expected wall-clock:

- `cargo test -p strategy --test vol_killswitch_overlay_end_to_end`: ~5s.
- `cargo test --workspace --features candle,realdata`: ~30-60s.
- `bash scripts/verify_anchors.sh`: ~60-90s (depends on disk).
- `cargo clippy --workspace --features candle,realdata`: ~30-60s incremental.

None of these are >2 min on a warm cache; if any is, the developer
captures the runtime in the wave-status update.

## Notes

- **Single Wave A**: the fix is small enough (Q1=(i) default,
  ~10-20 LoC) that splitting into Waves A/B/C (like the precedent
  did for the larger vol_targeting refactor) is over-scoped.
  Architect M-T1 confirms or splits per H3.
- **Forensic-gate bracket**: T-D-N1 (pre-fix RED) → T-D-N3 (post-
  fix GREEN) is the load-bearing evidence that the R6 unit test
  actually constrains the new behavior; mirror the
  v3-volatility-forecaster-noop-fix T-D-N3a/3b protocol.
- **Anchor delta**: ZERO by construction. The non-regression
  contract item 1 is the load-bearing claim; verify_anchors at
  M-FINAL is the gate.

## Changelog

- 2026-05-26 (analyst): tasks scaffold authored at HANDOFF.
  T-A1..T-A6 ticked. T-OD1..T-OD3 carry standing-Autoapprove
  defaults (Q1=(i), Q2=(a), Q3=(a)). T-AR / T-D-N / T-T / T-P
  rows pre-populated as a scaffold; architect refines at M-T1.
  Frontmatter status set to `in-progress` per the tasks-file
  convention.
