---
slug: vol-killswitch-overlay-noop-fix
status: in-progress
owner: tester
updated: 2026-05-26
priority: P0
---

# vol-killswitch-overlay-noop-fix — tasks

> Per AGENT.md, task IDs use these prefixes: **T-A** analyst,
> **T-OD** operator-decide, **T-AR** architect, **T-D-N** developer
> (Wave A), **T-T** tester, **T-P** presenter.
>
> **M-T1 status (2026-05-26):** H1 falsification probe executed.
> **H1 REFUTED.** The analyst's hypothesis (the
> `if sig.symbol == bar.symbol` filter at line 236 is the bug) is
> WRONG. Probe surfaced the actual root cause: the inner
> `MomentumStrategy::on_bar` never emits any signals across the
> entire test run (`base_signal_count = 0` for every bar, including
> the trigger bar). The kill-switch overlay's filter is correct in
> shape; there's nothing to mutate because the inner strategy hasn't
> warmed up. See T-AR-OD-1 for the escalation to operator-decide.

## T-A — Analyst (M0 — closed at HANDOFF)

| ID | Owner | Milestone | Depends on | Blocks | file:line | Acceptance |
|----|-------|-----------|------------|--------|-----------|------------|
| T-A1 | analyst | M0 | — | T-OD1..T-OD3 | `spec/vol-killswitch-overlay-noop-fix/feature.md` | feature.md v0.1.0 authored with R1-R6 + K1-K6 + H1-H3 + Q1-Q3 + non-regression contract + verdict tree + cost framing. |
| T-A2 | analyst | M0 | T-A1 | T-AR1 | `spec/vol-killswitch-overlay-noop-fix/tasks.md` | tasks.md authored with M0 / M-OD / M-T1 / M-DEV / M-FINAL / M-PRESENTER scaffold. |
| T-A3 | analyst | M0 | T-A1 | M-T1 entry | `spec/backlog.md ## Active` | Active row appended citing Bug #65 + safety framing. |
| T-A4 | analyst | M0 | T-A1 | M-FINAL trace flip | `spec/trace.toml` (end) | REQ-VOL-KILLSWITCH-NOOP-FIX-001 row appended at the END of trace.toml in `proposed` state. Does NOT modify any existing row (parallel architect owns REQ-REFLECTION-TRADER-001 at line 1084). |
| T-A5 | analyst | M0 | T-A1 | bug-log close | `spec/bug-log.md` § #65 | Bug log #65 row updated to `Status: open (analyst brief authored)` + cross-link to `spec/vol-killswitch-overlay-noop-fix/feature.md`. |
| T-A6 | analyst | M0 | T-A1..T-A5 | M-OD | — | Verify gates: `scripts/spec_lint.py` no new errors; `bash scripts/verify_anchors.sh` PASS (34/34). |

All T-A* rows ticked at HANDOFF (analyst's pass closed 2026-05-26).

- [x] **T-A1** — feature.md authored.
- [x] **T-A2** — tasks.md authored.
- [x] **T-A3** — Active block in `spec/backlog.md` appended.
- [x] **T-A4** — REQ row appended at END of `spec/trace.toml`.
- [x] **T-A5** — `spec/bug-log.md` § #65 row updated to `open (analyst brief authored)`.
- [x] **T-A6** — Hard gates verified: spec_lint inherits existing drift only; verify_anchors PASS (34/34).

## T-OD — Operator-decide (Q1..Q3, M-OD)

Standing Autoapprove from the v3-volatility-forecaster-noop-fix
2026-05-22 precedent applied to the analyst-recommended defaults at
M-OD (pre-M-T1). **NOTE: these defaults are now stale given M-T1's H1
refutation; the operator must re-decide Q1 via T-AR-OD-1 below.**

| ID | Owner | Milestone | Depends on | Blocks | Default | Resolution |
|----|-------|-----------|------------|--------|---------|------------|
| T-OD1 | orchestrator | M-OD | T-A6 | T-AR1 | **Q1=(i)** mutate `Signal::kind = Hold` on trigger | ~~Standing Autoapprove ticked~~ **SUPERSEDED by T-AR-OD-1** (architect M-T1 H1 refutation). |
| T-OD2 | orchestrator | M-OD | T-A6 | T-AR1 | **Q2=(a)** zero new anchors at v0.1.0 | Standing Autoapprove ticked. Still applies — zero anchor delta regardless of fix shape. |
| T-OD3 | orchestrator | M-OD | T-A6 | T-AR1 | **Q3=(a)** defer trait-shape decision | Standing Autoapprove ticked. Still applies — no trait surface change at v0.1.0. |
| T-OD4 | orchestrator | M-OD | T-OD1..T-OD3 | M-T1 entry | — | Frontmatter flipped `status: draft → in-progress`, `owner: analyst → architect`. spec/trace.toml state flipped `proposed → in-progress`. |

## T-AR — Architect (M-T1)

| ID | Owner | Milestone | Depends on | Blocks | file:line | test cmd | Expected output / Result |
|----|-------|-----------|------------|--------|-----------|----------|--------------------------|
| T-AR-0 | architect | M-T1 | T-OD4 | T-AR-1 | `crates/strategy/src/vol_killswitch_overlay.rs:229` + `crates/strategy/tests/vol_killswitch_overlay_end_to_end.rs:169` | (probe) `eprintln!` injection at top of overlay's basket-mutation block + un-#[ignore] `trigger_fires_and_equity_diverges`; `cargo test -p strategy --test vol_killswitch_overlay_end_to_end trigger_fires_and_equity_diverges -- --nocapture` | **DONE 2026-05-26.** H1 **REFUTED.** Probe output captured for all 64 calls (32 bars × 2 symbols): `base_signal_count = 0` on every call, including the trigger bar (`kill_active=true bar_symbol=BTCUSDT base_signal_count=0`). The inner `MomentumStrategy` never emits any signals across the entire test run. `kill_switch_count` advances to 2 correctly. The filter `if sig.symbol == bar.symbol` is structurally irrelevant when there are no signals to mutate. **Probe code REVERTED 2026-05-26** (`git diff` clean on both files; `#[ignore]` annotations restored at lines 169 + 237). |
| T-AR-OD-1 | architect → operator | M-T1 | T-AR-0 | T-AR-1 | feature.md § Q1 (analyst-authored) | — | **ESCALATION TO OPERATOR.** H1 is refuted; the analyst's Q1=(i) "mutate `Signal::kind = Hold` and drop the filter" no longer addresses the root cause. The bug is in the **test fixture**, not the production overlay. New options Q4=(p1)/(p2)/(p3) below — operator chooses. See § Q4 below the table. |
| T-AR-1 | architect | M-T1 | T-AR-OD-1 | T-D-N1 | (locked at T-AR-OD-1 below) | — | Lock the fix shape per operator Q4 resolution. Default recommendation: **Q4=(p1) fix the test fixture** (extend WARMUP_BARS to ≥ 65 so momentum strategy's ring buffer fills + first rebalance fires); leave the overlay source untouched. ~15 LoC change in 1 file. |
| T-AR-2 | architect | M-T1 | T-AR-1 | T-D-N4 | `spec/anchors.toml` | `grep "vol_killswitch\|vol-killswitch" /Users/Vitaliy.Schreibmann/Projects/Privat/trading/trading/spec/anchors.toml` | **DONE 2026-05-26.** ZERO matches. Q2=(a) holds: zero anchor delta by construction. |
| T-AR-3 | architect | M-T1 | T-AR-1 | T-D-N1 | feature.md § Q3 | — | **DONE 2026-05-26.** Q3=(a) confirmed: defer `Strategy::dampen_signals` trait surface to v0.1.1+. The Q4=(p1) test-fixture fix does NOT touch the trait surface; the Q4=(p2) inner-strategy warmup fix does NOT touch the trait surface; Q4=(p3) "do both" does NOT touch the trait surface. Trait surface is fully out of scope at v0.1.0 regardless of Q4. |
| T-AR-4 | architect | M-T1 | T-AR-1 | T-D-N1 | (no ADR amendment) | — | **DONE 2026-05-26.** No ADR amendment required. ADR-0038 § D6.b "wiring-bug-fix re-emission protocol" applies trivially (zero anchor delta; empty enumeration satisfies "affected anchors" clause). Since the bug is test-fixture-only, not production-code, the protocol is strictly trivial — no source mutation, no anchor mutation. |
| T-AR-5 | architect | M-T1 | T-AR-1..T-AR-4 | M-DEV entry | feature.md § Design + tasks.md frontmatter | — | § Design block appended with cross-pointer to this tasks.md § T-AR-OD-1 (Q4 escalation). Frontmatter flips `owner: architect → developer` AFTER Q4 resolution. trace.toml `arch` column populated with this tasks.md path + the Q4-resolution route. |

### T-AR-OD-1 — Q4 escalation (operator-decide, NEW at M-T1)

H1 is refuted. The analyst's three Q1 options addressed the wrong root cause. The actual root cause is:

> The inner `MomentumStrategy::on_bar` never emits any signals across the test run because the ring buffer (`capacity = lookback_minutes + 1 = 61`) never fills. The test pushes `WARMUP_BARS + 1 + POST_SPIKE_BARS = 31` bars per symbol — about half of what's needed. `is_rebalance_bar()` returns false at every call because `all_warmed()` is never true → `return Vec::new()` → nothing to mutate. The overlay's `kill_switch_count` correctly increments (the trigger arithmetic is sound), but `base_signals.into_iter().map(...).collect()` operates on an empty vector.

The kill-switch overlay's filter `if sig.symbol == bar.symbol { sig.kind = Hold }` at line 236 is **structurally correct** for the cross-sectional-basket-only semantic (K2 in the brief): when BTCUSDT's bar triggers the kill, the overlay converts BTCUSDT's Buy/Sell to Hold; other basket symbols pass through with whatever the inner strategy emitted for them. The bug is upstream — the test never lets the inner strategy emit.

Operator chooses one of:

| Q4 Option | Action | Cost | Pros | Cons | Architect recommendation |
|-----------|--------|------|------|------|--------------------------|
| **(p1)** | **Fix the test fixture.** Extend `WARMUP_BARS` from 20 → 65 (lookback_minutes=60, capacity=61). Use shorter `lookback_minutes` (e.g. 5) in the test's stub momentum config so warmup completes in ~10 bars while still being a valid rebalance test. Leave overlay source untouched. | ~1 day. ~15 LoC change in `crates/strategy/tests/vol_killswitch_overlay_end_to_end.rs`. Zero source code change. | Smallest blast radius; surgically targets the actual root cause; preserves overlay's correct cross-sectional-basket semantic; no anchor delta; no trait change. | Does NOT add a second consumer-of-the-overlay test for the case where the operator wants the trigger-symbol-only Hold conversion to actually fire on a populated signal stream — but the brief's R6 unit test can drive that with a stub inner-strategy emitter. | **DEFAULT.** Matches the orchestrator's "test-fixture bug" diagnosis path; lowest cost; closes Bug #65 cleanly. |
| **(p2)** | **Treat as a real production semantic gap.** Broaden the overlay's filter to `if kill_active { ALL signals → Hold }` (drop the per-symbol filter entirely). This converts the kill-switch from "kill the trigger symbol's basket-position signals" to "kill the whole basket when any symbol trips." | ~2-3 days. ~10 LoC in overlay + ~20 LoC tests + spec update for the semantic shift. | Stronger killswitch semantic ("kill everything on regime spike"); arguably safer in production. | Changes a production semantic the analyst's K2 explicitly identifies as out-of-scope ("kill-trigger-symbol-only"); risks over-suppressing on bars where only one of N basket symbols spikes; expands LoC + test count; requires re-reading the operator's K2 framing. | Rejected by default — out of v0.1.0 scope per K2; would need an operator semantic re-decision. |
| **(p3)** | **Do both (p1) + (p2).** Fix the test fixture + broaden the filter. | ~3-5 days. | Most defensive; covers both the test gap and the production semantic question. | Highest cost; risks shipping a semantic change the operator did not ask for; widens K2's scope unilaterally. | Rejected — over-scopes a P0 wiring-bug recovery into a semantic refactor. |

**Architect recommendation: Q4=(p1) — fix the test fixture only.**

**Rationale**: the smoking gun (per-signal filter at line 236) is structurally correct for the kill-trigger-symbol-only semantic the analyst's K2 explicitly locked. The test was authored against an unrealistic warmup-skipped premise. Fixing the test fixture surfaces both:
- (a) the overlay's filter actually works correctly when there are signals to mutate (R4 `post_trigger_signals_are_hold` flips RED → GREEN), AND
- (b) the equity divergence appears (R3 `trigger_fires_and_equity_diverges` flips RED → GREEN) because the basket now has BTCUSDT-Buy at rebalance bars that the overlay correctly suppresses to Hold on the trigger bar.

Both currently `#[ignore]`-gated tests would pass under Q4=(p1) WITHOUT any production source change. This is the cheapest, narrowest fix path. **Standing Autoapprove should be re-extended to Q4=(p1)** under the same pattern as the analyst's Q1..Q3 defaults.

### T-AR-OD-1 verification — **operator must tick before M-DEV opens**

- [x] **T-OD-Q4** — operator chose **(p3) Both — fix test + broaden overlay** at 2026-05-26 ("(p3) Both — fix test + broaden overlay" verbatim). Architect-default (p1) was rejected in favour of the defensive belt+suspenders path: the test fixture is fixed AND the overlay's per-signal `sig.symbol == bar.symbol` filter at line 236 is broadened so cross-sectional basket signals also get dampened when the killswitch trips for ANY symbol in the basket. Cost rises from ~1 day → ~3-5 days. Architect MUST re-spawn (or the orchestrator must inline-amend) M-DEV's wave shape — current "Wave A single wave, ~1 day under Q4=(p1)" wording below is SUPERSEDED. Bug-log #65 closes only when both halves land.

## T-D-N — Developer (Wave A — Q4=(p3) "Both": fixture fix + broadened filter)

Wave shape amended at operator Q4=(p3) decision. Developer implemented BOTH halves.

| ID | Owner | Milestone | Depends on | Blocks | file:line | test cmd | Expected output |
|----|-------|-----------|------------|--------|-----------|----------|-----------------|
| T-D-N1 | developer | M-DEV | T-AR-5 (Q4 resolved) | T-D-N2 | (forensic only — pre-fix captures in earlier context) | — | **Forensic pre-fix captured.** Both `trigger_fires_and_equity_diverges` and `post_trigger_signals_are_hold` were `#[ignore]`-gated and RED. Pre-fix evidence archived in context summary. |
| T-D-N2 | developer | M-DEV | T-D-N1 | T-D-N3 | `crates/strategy/tests/vol_killswitch_overlay_end_to_end.rs:78-94` | — | **A.1: Fix test fixture.** `lookback_minutes` changed 60→5. Flat BTC warmup added (prevents GARCH early-kill with `min_median_floor=1e-3`). Two-spike scenario designed. |
| T-D-N3 | developer | M-DEV | T-D-N2 | T-D-N4 | `crates/strategy/src/vol_killswitch_overlay.rs:231-244` | — | **A.2: Broaden overlay filter.** Dropped `if sig.symbol == bar.symbol` guard; all signals converted to Hold when `kill_active`. Q4=(p3) broadened semantic. |
| T-D-N4 | developer | M-DEV | T-D-N3 | T-D-N5 | `crates/strategy/tests/vol_killswitch_overlay_end_to_end.rs:224-289,294-361,364-418,420-511` | `cargo test -p strategy --test vol_killswitch_overlay_end_to_end` | **A.3: Remove `#[ignore]` + add basket test.** All 4 tests pass. See output below. |
| T-D-N5 | developer | M-DEV | T-D-N4 | T-D-N6 | `crates/strategy/tests/vol_killswitch_overlay_end_to_end.rs:364-418` | `cargo test -p strategy --test vol_killswitch_overlay_end_to_end passthrough_when_threshold_unreachably_high` | **Negative control.** `test passthrough_when_threshold_unreachably_high ... ok` |
| T-D-N6 | developer | M-DEV | T-D-N5 | T-D-N7 | `spec/anchors.toml` | `bash scripts/verify_anchors.sh` | **Anchor check** — see T-D-N6 note below. |
| T-D-N7 | developer | M-DEV | T-D-N6 | T-D-N8 | workspace | `cargo clippy -p strategy --all-targets -- -D warnings` + workspace tests | **Workspace sweep** — see T-D-N7 note below. |
| T-D-N8 | developer | M-DEV | T-D-N7 | M-FINAL entry | `spec/bug-log.md` § #65 | — | **A.4: Bug-log updated** — `Status: open → FIXED 2026-05-26`. Changelog appended. |

### Developer task verification citations

**T-D-N4** (4 tests PASS):
- file:line: `crates/strategy/tests/vol_killswitch_overlay_end_to_end.rs` (all 4 tests)
- test cmd: `cargo test -p strategy --test vol_killswitch_overlay_end_to_end`
- output: `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s`

**T-D-N4 / overlay hygiene gate** (2 tests PASS):
- file:line: `crates/strategy/tests/overlay_hygiene_gate.rs` — `vol_killswitch_overlay` removed from `KNOWN_UNCOVERED`
- test cmd: `cargo test -p strategy --test overlay_hygiene_gate`
- output: `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s`

**T-D-N7** (clippy PASS):
- file:line: `crates/strategy/` all targets
- test cmd: `cargo clippy -p strategy --all-targets -- -D warnings`
- output: `Finished 'dev' profile [unoptimized + debuginfo] target(s)` (0 errors, 0 warnings)

**T-D-N8** (bug-log updated):
- file:line: `spec/bug-log.md:109`
- test cmd: n/a (spec file edit)
- output: Status line reads `FIXED 2026-05-26`

## T-T — Tester (M-FINAL)

| ID | Owner | Milestone | Depends on | Blocks | file:line | test cmd | Expected output |
|----|-------|-----------|------------|--------|-----------|----------|-----------------|
| T-T1 | tester | M-FINAL | T-D-N8 | T-T2 | workspace | `cargo fmt --check` | PASS (no output — clean). |
| T-T2 | tester | M-FINAL | T-T1 | T-T3 | workspace | `cargo clippy --workspace --features candle,realdata -- -D warnings` | `Finished 'dev' profile [unoptimized + debuginfo] target(s) in <Ns>` (0 warnings). |
| T-T3 | tester | M-FINAL | T-T2 | T-T4 | workspace | `cargo test --workspace --features candle,realdata` | All workspace tests PASS; workspace fail count delta = -2 ignored (the 2 `#[ignore]`'s removed); zero new test failures vs. last tester whitelist. |
| T-T4 | tester | M-FINAL | T-T3 | T-T5 | `crates/strategy/tests/vol_killswitch_overlay_end_to_end.rs` | `cargo test -p strategy --test vol_killswitch_overlay_end_to_end` | `test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`. Confirms R2 + R3 + R4 all PASS; the 2 `#[ignore]` annotations removed cleanly. |
| T-T5 | tester | M-FINAL | T-T4 | T-T6 | `spec/anchors.toml` | `bash scripts/verify_anchors.sh` | `ANCHORS PASS  (34 / 34)` — 34 anchors byte-identical (zero delta; vol_killswitch does not appear in any anchored scenario). Negative invariant confirmed. |
| T-T6 | tester | M-FINAL | T-T5 | M-PRESENTER entry | `spec/vol-killswitch-overlay-noop-fix/reports/test-final-<YYYY-MM-DD>.md` (NEW) | — | Test report written following `.claude/skills/rust-test/templates/test-report.md`. Verdict: **PASS**. Carries all cargo outputs + verify_anchors PASS + the 3 e2e test results + non-regression contract attestation + the H1-refuted / Q4=(p1) test-fixture-fix narrative. Frontmatter flips `owner: tester → presenter`. trace.toml `anchors` column populated ("34/34 PASS"). |

## T-P — Presenter (M-PRESENTER)

| ID | Owner | Milestone | Depends on | Blocks | file:line | Acceptance |
|----|-------|-----------|------------|--------|-----------|------------|
| T-P1 | presenter | M-PRESENTER | T-T6 | M-OPERATOR | `spec/vol-killswitch-overlay-noop-fix/presentations/vol-killswitch-overlay-noop-fix-<YYYY-MM-DD>.md` (NEW) | Operator-approval deck assembled. Carries: (1) Bug #65 framing + safety severity; (2) **H1 refuted / Q4=(p1) test-fixture fix narrative** — the actual root cause was the inner strategy's warmup gate never opening due to undersized WARMUP_BARS; the overlay's per-signal filter is structurally correct; (3) the 3-cell verdict tree from feature.md § Pre-drawn verdict routing tree; (4) the 1 PASS → 3 PASS test count delta; (5) 34/34 anchors byte-identical attestation; (6) recommend **R-O1 → SHIP** path under standing Autoapprove. HANDOFF → orchestrator → operator-approve. |
| T-P2 | presenter | M-PRESENTER | T-P1 | M-OPERATOR | `spec/dev-notes/bug-65-test-fixture-warmup-discovery-<date>.md` (NEW) | Dev-note authored documenting the H1 refutation evidence + the test-fixture warmup gate as the future-debugging signal for similar "overlay no-op" symptoms. ~50-100 lines. Sibling of `v3-vol-overlay-noop-discovery-2026-05-22.md`. |
| T-P3 | presenter | M-PRESENTER | T-P2 | M-OPERATOR | `spec/architecture.md` § "Strategy crate" subsection (small append) | — | Document the warmup-gate ergonomic in the strategy crate's architectural notes — any future overlay e2e test MUST ensure WARMUP_BARS ≥ lookback_minutes + 1, OR use a short-lookback stub config. Single paragraph addition. |

## Watch recipes (per MEMORY.md)

If any cargo step at M-DEV / M-FINAL runs > 2 min, the developer
emits a copy-pasteable `watch -n 2 '<probe>'` block in the wave-
status update so the orchestrator can stream progress without
polling. Expected wall-clock:

- `cargo test -p strategy --test vol_killswitch_overlay_end_to_end`: ~1-5s (post-fix; with proper warmup the bars run quickly).
- `cargo test --workspace --features candle,realdata`: ~30-60s.
- `bash scripts/verify_anchors.sh`: ~60-90s (depends on disk).
- `cargo clippy --workspace --features candle,realdata`: ~30-60s incremental.

None of these are >2 min on a warm cache.

## Notes

- **H1 REFUTED.** The architect M-T1 probe surfaced that `MomentumStrategy::on_bar` never emits any signals — `base_signal_count = 0` for every call, including the trigger bar. The overlay's per-signal symbol filter is structurally correct; the bug is upstream in the test fixture's undersized warmup.
- **Cost re-estimate (Q4=(p1) default):** ~1 day (matches the analyst's lower-bound estimate). The fix is a single-line constant change + the 2 `#[ignore]` removals + workspace gates. If operator picks Q4=(p2) or (p3), the cost rises to 2-5 days.
- **Single Wave A**: the fix is small enough that splitting into Waves A/B/C is over-scoped.
- **Forensic-gate bracket**: T-D-N1 (pre-fix RED capture) → T-D-N3 (post-fix GREEN capture) is the load-bearing evidence that the test now meaningfully constrains the overlay's behavior.
- **Anchor delta**: ZERO by construction. Verified at T-AR-2.
- **Production source untouched** (under Q4=(p1) default). The overlay's `on_bar` body stays byte-identical from main; only the test file changes. This is unusual for a P0 wiring-bug fix — but matches the actual diagnosis: the bug is in the test's fixture, not the production code.

## Changelog

- 2026-05-26 (analyst): tasks scaffold authored at HANDOFF. T-A1..T-A6 ticked. T-OD1..T-OD3 carry standing-Autoapprove defaults (Q1=(i), Q2=(a), Q3=(a)). T-AR / T-D-N / T-T / T-P rows pre-populated as a scaffold; architect refines at M-T1. Frontmatter status set to `in-progress` per the tasks-file convention.
- 2026-05-26 (architect): M-T1 H1 falsification probe complete. **H1 REFUTED.** Probe surfaced `base_signal_count = 0` on every call — the inner `MomentumStrategy::on_bar` never emits signals in the test because the ring buffer (capacity 61) never fills with the test's 31 bars per symbol. Filter at line 236 is structurally correct; bug is in the test fixture's undersized WARMUP_BARS / lookback_minutes pairing. T-AR-OD-1 escalated to operator-decide as Q4. Default recommendation Q4=(p1) — fix the test fixture; leave overlay source untouched. T-OD1 (Q1) superseded by Q4. T-AR-1..T-AR-5 ticked. Probe code REVERTED before this commit (`git diff` clean on both files; `#[ignore]` annotations restored at lines 169 + 237; verify_anchors PASS 34/34). HANDOFF → operator-decide (Q4) → developer.
- 2026-05-26 (developer): Wave A complete — Q4=(p3) "Both" implemented. A.1: test fixture fixed (lookback_minutes 60→5, flat BTC warmup, two-spike bar stream, min_median_floor=1e-3). A.2: overlay filter broadened to basket-wide Hold (dropped per-symbol guard). A.3: #[ignore] removed; broadened_filter_dampens_cross_sectional_basket test added; 4/4 e2e tests pass; overlay hygiene gate 2/2 pass; clippy clean. A.4: bug-log #65 status → FIXED. HANDOFF → tester.
