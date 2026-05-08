---
title: Test Report
feature: <feature-slug>
run_id: 2026-05-07-1330-UTC
commit: 3efda6401e187db2a5bf9c21d83a0cbf862071f0
agent: tester
verdict: FAIL
---

# Test Report — <feature-slug> — 2026-05-07 13:30 UTC (first pass)

> **R16.3 self-check note.** Per Brief R16.3, four brand-bleed
> tokens (the design-system name plus the three tier/elevation
> tokens listed in the gate-5 grep pattern) must not appear in
> `spec/reports/` test-/backtest- bodies. This report deliberately
> elides those four literals in prose. Task-list and feature-brief
> paths are referred to by the placeholder `<feature-slug>` (or
> `<phase-1-slug>` / `<phase-2-slug>` / `<phase-3-slug>` /
> `<phase-4-slug>` / `<master-roadmap>` as appropriate) wherever the
> literal slug would otherwise leak into report content. The
> four-token grep regex itself is never reproduced as a contiguous
> string in this report body — § 2 / § 7 / § 8 refer to it as "the
> Brief R16.3 four-token grep" instead, the same elision pattern
> Phase 1 / 2 / 3 / 4 testers used.

## 1. Scope

- **Feature / change under test:** Phase 5 HumanControl + AgentFeed
  rename — first net-new operator-write surfaces in the cockpit:
  the `HumanControl` panel widget (mode segmented control + 3 limit
  mirror rows + kill bottom action) at the new 7th sidebar entry,
  per-strategy pause/resume button on the Strategies-detail screen
  + Home → Strategies-summary panel, per-veto override-risk-veto
  control with `OVERRIDE` typed-confirm modal flow, the
  Observe / Supervised / Auto execution-mode toggle (runtime-only),
  the four-phase TD-1 closure via the new `widgets::focus_ring`
  custom-widget escape hatch (path b), two new
  `StrategyEventKind` variants (`StrategyPaused` +
  `RiskVetoOverridden`) with sibling-of-`kill_switch_tripped`
  audit writers (no SQL migration; `kind` column is `TEXT`), and
  the module-level `tape` → `agent_feed` rename (9 baseline
  filenames renamed via `git mv`; `Cockpit::tape` field name
  preserved per Q14).
- **Spec refs:** `spec/features/<feature-slug>.md`,
  `spec/tasks/<feature-slug>.md`,
  `spec/features/<master-roadmap>.md`.
- **Commit SHA:** `3efda6401e187db2a5bf9c21d83a0cbf862071f0`
  (worktree carries uncommitted Phase 5 edits per task-list
  T1901–T1916 + the T1913 ui-designer attestation sub-block).
- **Rust toolchain:** `rustc 1.94.1 (e408947bf 2026-03-25)`.
- **OS / arch:** `Darwin 25.4.0 arm64` (M-series).
- **Predecessor reports:**
  - Phase 1 third-pass PASS:
    `spec/reports/test-2026-05-04c-<phase-1-slug>.md`.
  - Phase 2 first-pass PASS:
    `spec/reports/test-2026-05-05-<phase-2-slug>.md`.
  - Phase 3 first-pass PASS:
    `spec/reports/test-2026-05-05-<phase-3-slug>.md`.
  - Phase 4 second-pass PASS:
    `spec/reports/test-2026-05-06b-<phase-4-slug>.md`.
- **Run id:** `test-2026-05-07-<feature-slug>` — first-pass run.
- **Predecessor pipeline state.** Developer pass ticked T1901–T1916
  in one run (`cargo fmt --all -- --check` claimed clean per
  task-list T1916 acceptance line 1256–1257). ui-designer attestation
  pass (2026-05-07) wrote 13 net-new snapshot tests in
  `crates/ui/tests/panel_snapshots.rs` plus 4 helper summary
  functions, ran `cargo insta test -p ui --test panel_snapshots
  --accept` (67 pass), and signed the T1913 attestation sub-block.
  Orchestrator re-ran the rustdoc gate that was sandbox-blocked at
  the developer pass (`Finished dev profile … in 24.74s`,
  per the prompt brief).

## 2. Static Analysis

| Check | Result | Notes |
|-------|--------|-------|
| `cargo fmt --all -- --check` | **FAIL** | Exit 1. Eight diff
hunks across **two files** that the ui-designer's T1913
attestation pass authored without re-running fmt: one hunk in
`crates/ui/src/widgets/human_control.rs:202` (the `limit_row` fn
signature got line-broken across four lines but rustfmt prefers
single-line under the column width — net 5 → 1 lines), and seven
hunks in `crates/ui/tests/panel_snapshots.rs` (lines 731, 779,
1332, 1446, 1477, 1516, 1572 — six `assert_snapshot!` /
`format!` calls that got broken across multiple lines unnecessarily
plus one `let input_focused = …` chain). The diff is **whitespace-
mechanical** (no semantic change); a single `cargo fmt --all` over
the worktree would clear it. The developer's task-list claim at
T1916 acceptance line 1256–1257 (`cargo fmt --all -- --check`
"clean (no diff)") was true at the moment the developer wrote it,
but **stale by the time the ui-designer's attestation pass added 13
new tests + 4 helper summary functions** — the ui-designer pass
ran `cargo insta test` but did not re-run `cargo fmt`. § 2.1
quotes the diff verbatim. |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | PASS | `Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 1.40s`, zero warnings, exit 0. Independently re-run from project root post-developer-pass + post-ui-designer-pass; clippy converges with the cached compile graph. The orchestrator pre-cleared this gate; tester re-run confirms. |
| `cargo audit` | N/A (not installed) | `cargo audit` not on PATH (`error: no such command: 'audit'`); same handling as Phase 1 / 2 / 3 / 4 reports. Coverage gap is bridged by `cargo deny check` (`[advisories]` table v2 against the same RustSec DB) which PASSES. |
| `cargo deny check` | PASS | `advisories ok, bans ok, licenses ok, sources ok` — independent re-run from project root. |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | PASS | Tester re-ran cleanly from project root after `rm -rf target/doc`: `Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 21.33s`; `Generated /Users/Vitaliy.Schreibmann/Projects/Privat/trading/trading/target/doc/agent/index.html and 16 other files`. Zero warnings, zero errors. The orchestrator pre-cleared this gate at 24.74s after the developer-pass sandbox block; tester independently re-runs and converges. |

### 2.1 Fmt diff (verbatim — first 4 hunks of 8)

```
Diff in /Users/Vitaliy.Schreibmann/Projects/Privat/trading/trading/crates/ui/src/widgets/human_control.rs:202:
-fn limit_row(
-    label: &str,
-    value: String,
-    sentiment: Option<iced::Color>,
-) -> Element<'_, Message> {
+fn limit_row(label: &str, value: String, sentiment: Option<iced::Color>) -> Element<'_, Message> {

Diff in /Users/Vitaliy.Schreibmann/Projects/Privat/trading/trading/crates/ui/tests/panel_snapshots.rs:731:
 fn human_control__observe_default() {
     let c = Cockpit::new();
-    assert_snapshot!(
-        "human_control__observe_default",
-        human_control_summary(&c)
-    );
+    assert_snapshot!("human_control__observe_default", human_control_summary(&c));
 }

Diff in /Users/Vitaliy.Schreibmann/Projects/Privat/trading/trading/crates/ui/tests/panel_snapshots.rs:779:
     let mut c = Cockpit::new();
     c.execution_mode = ExecutionMode::Observe;
     c.risk_state = PanelState::Loading;
-    assert_snapshot!(
-        "human_control__limits_loading",
-        human_control_summary(&c)
-    );
+    assert_snapshot!("human_control__limits_loading", human_control_summary(&c));
 }

Diff in /Users/Vitaliy.Schreibmann/Projects/Privat/trading/trading/crates/ui/tests/panel_snapshots.rs:1332:
-    out.push_str(&format!(
-        "title: {}\n",
-        strings::PANEL_HUMAN_CONTROL_TITLE
-    ));
+    out.push_str(&format!("title: {}\n", strings::PANEL_HUMAN_CONTROL_TITLE));
```

Hunks 5–8 are at `panel_snapshots.rs:1446 / :1477 / :1516 / :1572`
and follow the same whitespace-mechanical pattern (multi-line
`format!` / `assert_snapshot!` calls + one chained `let
input_focused = …` re-flow). Net change is purely cosmetic —
behaviour and test output are bit-identical. A single
`cargo fmt --all` over the worktree clears all eight hunks.

## 3. Unit & Integration Tests

`cargo test --workspace --all-targets` — exit 0, all suites green.

| Metric | Value |
|---|---|
| Test binaries run | 110 |
| Tests passed | **896** |
| Tests failed | **0** |
| Tests ignored | 3 |
| Failing tests | _none_ |

Phase 4 → Phase 5 delta: **+46 tests / +2 binaries** (850/108 →
896/110). The Phase 5 net-new test families all converge clean:

### Spotlight tests (per Brief V-items + Phase 5 net-new tests)

| Test | Result | Output line |
|------|--------|-------------|
| `audit::journal::tests::strategy_paused_*` (4 unit, T1902 / V1) | PASS | All 4 inside `audit` lib. |
| `audit::journal::tests::risk_veto_overridden_*` (3 unit, T1902 / V2) | PASS | All 3 inside `audit` lib. |
| `audit::tests::strategy_paused` (integration, T1902) | PASS | Surfaced as the `strategy_paused` integration test binary. |
| `audit::tests::risk_veto_overridden` (integration, T1902) | PASS | Surfaced as the `risk_veto_overridden` integration test binary. |
| `core::strategy_events::tests::*` (Display + serde, T1902 / V3) | PASS | New variants emit `StrategyPaused` / `RiskVetoOverridden` PascalCase strings; round-trip through serde. |
| `state::tests::execution_mode_*` (T1901 / V4) | PASS | Inside `ui` lib `state` module. |
| `state::tests::strategy_pause_*` + `override_risk_veto_*` (T1901 / V5) | PASS | Inside `ui` lib `state` module. |
| `widgets::human_control::tests::*` (T1904, T1905, T1911) | PASS | Inside `ui` lib. |
| `widgets::override_risk_veto::tests::*` (T1909) | PASS | Inside `ui` lib. |
| `widgets::focus_ring::tests::*` (T1912 / TD-1 closure) | PASS | `focus_traversal_*` + `focus_halo_renders_on_focused` inside `ui` lib. |
| `pause_strategy_round_trip` (integration, T1908) | PASS | `crates/ui/tests/strategies_pause_round_trip.rs`. |
| `override_risk_veto_round_trip` (integration, T1910) | PASS | `crates/ui/tests/override_risk_veto_round_trip.rs`. |
| `panel_snapshots` suite (T1913) | PASS | `test result: ok. 67 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s` — 13 net-new + 9 renamed (`agent_feed_*`) + 1 Q1-driven Debug-without-kill regen. |
| `agent_feed_*` rename body diff | PASS | All 9 baselines under `crates/ui/tests/snapshots/panel_snapshots__agent_feed_*.snap` carry `panel: agent_feed` + `title: Agent activity` headers; row content unchanged; previous `tape_*` baselines are gone (verified: `find … -name 'panel_snapshots__tape_*.snap'` returns empty). |
| Cross-feature: `live_subscription_full_bus` (V14) | PASS | `t911_full_bus_drives_every_panel_out_of_loading`, `t911_kill_button_round_trip_via_mode_forwarder` — 2 passed. |
| Cross-feature: `cockpit_live_modal_metadata_chain` (V14) | PASS | 2 passed. |
| Cross-feature: `tape_row_click_opens_modal` (V14) | PASS | 8 passed. (Test-binary filename retains `tape_*` prefix because the audit-modal feature predates the rename and was deliberately not renamed — only the live-fills feed module path renamed; Q14 framing.) |
| Cross-feature: `recent_fills_filtered` (v1.5b multi-venue) | PASS | 4 passed. |

### Failing Tests

_none — `cargo test --workspace --all-targets` exit 0, 896 / 0 / 3._

## 4. Property / Fuzz Tests

`proptest` is present in `core` (positive-qty test). Default-feature
run already exercises it (`prop_positive_qty_accepted` PASS in
`trading_core` lib). Larger budget run (`PROPTEST_CASES=1024`) was
not re-executed in this gate pass — same handling as Phase 1 / 2 /
3 / 4 testers; Brief 8-gate matrix did not request a budget bump.

| Suite | Cases | Shrunk failures | Seed |
|---|---:|---:|---|
| `core::prop_positive_qty_accepted` (default budget) | 256 | 0 | system-default |

## 5. Backtest Results

_n/a — Phase 5 ships the first net-new operator-write surfaces over
the audit ledger via additive `StrategyEventKind` variants
(`StrategyPaused`, `RiskVetoOverridden`); no schema migration
(`kind` column is already `TEXT`); no committed report body
re-renders; no new backtest scenarios. The 11 body-SHA-256 backtest
anchors are verified byte-identical via Gate 4 below._

## 6. Benchmarks

_n/a — Phase 5 changes are cockpit-side widgets + 2 audit writers +
the module rename + the focus-ring custom-widget escape hatch only;
no hot-path code touched (no order-book, feature-calc, or inference
changes). The two new audit writers go through the same insert path
as `kill_switch_tripped`; no new index._

## 7. Environment / Infrastructure Issues

- `cargo audit` not installed on PATH. Per the rust-validate skill:
  "Install with `cargo install cargo-audit` if missing; ask the
  user before installing." Tester role does not auto-install.
  Coverage gap is bridged by `cargo deny check`
  (`[advisories]` v2 against the same RustSec DB) which PASSES.
- The Brief R16.3 four-token grep run against `spec/reports/` with
  `--include='backtest-*.md' --include='test-*.md'` returns **zero
  matches** (exit 1 → reported as "0 lines" via `wc -l`). Self-
  check on this report file: brand-bleed tokens absent (see prelude
  / R16.3 self-check note).
- T1916 acceptance sub-bullet at task-list line 1276–1279 records
  the `cargo doc` gate as `sandbox-blocked (env-var argument
  denied)` at the developer pass — orchestrator pre-cleared
  (24.74s, per prompt brief); tester re-ran independently
  (21.33s). The orchestrator-side line itself is not embedded in
  the task-list T1916 sub-bullet; it lives in the orchestrator
  hand-off note. Not a gate failure (the gate PASSES on the tester
  re-run); flagged as a small honest-tick polish opportunity.
- No flaky tests observed; runtime is determinism-stable on the
  M-arm Darwin host.

## 8. Verdict

**`FAIL`**

Gate 3 (`rust-validate`) is RED on the **fmt** sub-step. Eight
whitespace-mechanical diff hunks across two ui-designer-authored
files (the T1913 attestation pass: `crates/ui/src/widgets/human_control.rs:202`
+ six `assert_snapshot!` / `format!` re-flows + one `let
input_focused = …` chain in `crates/ui/tests/panel_snapshots.rs`)
mean `cargo fmt --all -- --check` returns exit 1. The
ui-designer's pass ran `cargo insta test` but did not re-run
`cargo fmt`; the developer's earlier T1916-acceptance claim that
fmt was clean is now stale. Every other gate (clippy / deny /
rustdoc / 896 tests / 11 anchors / R16.3 / 7-of-7 cross-feature /
86-baselines-clean / ui-designer attestation) PASSES.

The fix is mechanical: a single `cargo fmt --all` over the
worktree clears all eight hunks. **Phase 5 is one fmt run away
from PASS.** Same precedent as Phase 4 first-pass: a single
static-analysis miss (Phase 4 was `clippy::match_same_arms`; Phase
5 is fmt-whitespace) routed to the developer for the trivial
fixup. Tester does not edit code (audit-only role); routes to
developer per the AGENT.md verdict map.

The four-phase TD-1 deferral closure (path b — custom-widget
escape hatch via `crates/ui/src/widgets/focus_ring.rs`) is
complete and visually evidenced
(`panel_snapshots__focus_ring__focused_kill_button.snap`); the
TD-2 row (risk-engine veto-emit upstream wiring) is appended at
`spec/features/<master-roadmap>.md:796–863`; both architect-side
master-roadmap edits are in place. The 13 net-new + 9 rename + 1
Debug-without-kill snapshot deltas all converge with zero
`*.pending-snap` files.

`T_FINAL_<phase-5-tag>` stays `[ ]`. Phase 5 brief frontmatter
stays `status: active`.

### Gate-by-gate summary

| # | Gate | Result | Note |
|---|------|--------|------|
| 1 | Honest-tick audit (T1901–T1916 + T1913 ui-designer attestation sub-block) | PASS | All 16 task ticks at task-list lines 159 / 245 / 344 / 425 / 467 / 512 / 577 / 623 / 667 / 723 / 779 / 847 / 943 / 1138 / 1197 / 1220 carry file:line + test cmd + output. T1913 visual-diff attestation sub-block at task-list line 989 carries the `_ticked 2026-05-07 (ui-designer)._` signature with 8/8 Q-evidence (Q1 / Q5 / Q6 / Q7 / Q8 / Q9 / Q12 / Q14) + TD-1 closure verification + `unknown`-color sweep. T1916 sub-bullet records `cargo doc` as sandbox-blocked at developer pass; orchestrator pre-cleared and tester independently re-ran (21.33s) — orchestrator-side `Finished … in 24.74s` line is not embedded in the T1916 sub-bullet (small polish opportunity, not a gate failure). T1916's fmt claim ("clean (no diff)") was true at developer-pass-time but is now **stale** as of the ui-designer's T1913 attestation pass (the FAIL surfaced at Gate 3 below). |
| 2 | `cargo test --workspace --all-targets` | PASS | All suites green: **896 passed, 0 failed, 3 ignored** across **110 test binaries**. Phase 5 net-new tests verified individually via spotlight table in § 3. |
| 3 | `rust-validate` (fmt + clippy + cargo-deny + audit + docs) | **FAIL** | **fmt FAIL** (8 whitespace-mechanical diff hunks in ui-designer-authored files at `crates/ui/src/widgets/human_control.rs:202` + `crates/ui/tests/panel_snapshots.rs:731 / :779 / :1332 / :1446 / :1477 / :1516 / :1572`; § 2.1 quotes hunks 1–4 verbatim); clippy PASS (`Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 1.40s`, zero warnings); deny PASS (`advisories ok, bans ok, licenses ok, sources ok`); audit N/A (not installed; deny advisories cover); rustdoc PASS (`Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 21.33s` after `rm -rf target/doc`). Single `cargo fmt --all` over the worktree clears all eight hunks. |
| 4 | `bash scripts/verify_anchors.sh` | PASS | `ANCHORS PASS  (11 / 11)` — all 11 body-SHA-256s byte-identical to `spec/anchors.toml`. Phase 5 introduces no anchor risk by construction (additive `StrategyEventKind` variants; no SQL migration; no committed report body re-renders). |
| 5 | R16.3 brand-bleed grep on `spec/reports/` | PASS | Targeted grep with `--include='backtest-*.md' --include='test-*.md'` returns zero matches in test- and backtest- report bodies. Self-check on this file: zero matches in body text (verified per the prelude elision contract). |
| 6 | Cross-feature invariants 7/7 PASS | PASS | Tester independently re-ran each prior feature's named test: (1) `operator-success-reports` — `cargo test -p reports csv_artifacts::tests --lib` → `4 passed`; (2) `live-cockpit-unified` — `cargo test -p ui --features live --test live_subscription_full_bus` → `2 passed`; (3) `real-mtm-unrealized-pnl` — `cargo test -p ui --lib widgets::pnl` → `0 passed; 0 failed; 0 ignored; 106 filtered out` (P&L card has no unit tests in `widgets::pnl::tests`; surface unchanged confirmed via panel `pnl_*` baselines remaining byte-identical at Gate 7); (4) `per-symbol-position-accounts` — `cargo test -p audit --lib query::tests::position` → `0 passed; 20 filtered out` (sibling read path `recent_fills_filtered` 4/4 PASS); (5) `tape-row-audit-modal` — `cargo test -p ui --features fixtures --test tape_row_click_opens_modal` → `8 passed`; (6) `journal-tx-metadata` — `cargo test -p ui --features live --test cockpit_live_modal_metadata_chain` → `2 passed`; (7) `v1.5b-multi-venue` — `cargo test -p audit --lib query::tests::recent_fills_filtered` → `4 passed`. Identical evidence pattern to T1914's developer block + Phase 4 tester block. |
| 7 | Snapshot baselines clean | PASS | `find crates/ui/tests/snapshots crates/ui/src/widgets/snapshots crates/audit/tests/snapshots -name '*.pending-snap' -o -name '*.snap.new'` returns empty. Total `*.snap` baseline count: **86** = **67** in `crates/ui/tests/snapshots/` panel-side (55 from Phase 4 + 12 net-new + 1 Q1-driven `debug_screen__without_kill` regen replacing the retired `debug_screen__full`; the 9 `agent_feed_*` renames are in-place via `git mv`) + **17** in `crates/ui/src/widgets/snapshots/` widget-side (unchanged from Phase 4) + **2** in `crates/audit/tests/snapshots/` audit-side (`risk_veto_overridden__strategy_events__risk_veto_overridden_row.snap` + `strategy_paused__strategy_events__strategy_paused_row.snap`, both NEW at T1902). Matches the ui-designer attestation count exactly. |
| 8 | Visual-diff attestation by ui-designer | PASS | T1913 visual-diff attestation sub-block at task-list line 989 carries the `_ticked 2026-05-07 (ui-designer)._` signature. 8/8 Q-evidence rolled up: Q1 (placement — HumanControl as 7th sidebar entry; Debug-screen kill removed) ✓; Q5 (TD-1 path b focus-ring overlay — `halo_visible: true`, zero `button::Status::Focused` hits in the codebase) ✓; Q6 (rename via `git mv` — title-string body diff only on the 9 agent_feed pairs) ✓; Q7 (full reference-component field set — three mirror rows present in all three mode baselines) ✓; Q8 (single-click pause both directions — `typed_confirm: false`) ✓; Q9 (per-veto override — modal contract matches kill-confirm visual) ✓; Q12 (kill copy preserved — `KILL_BUTTON_LABEL = "Stop trading"`; zero `Halt all agents` hits in `crates/ui/`) ✓; Q14 (`Cockpit::tape` field name preserved — module rename only) ✓. Sample-attested rows + full-inventory verification (86 baselines) + `unknown`-color sweep (zero unmapped beyond the legitimate `Latency::Unknown` badge) all preserved. |

## 9. Routing

`HANDOFF → developer` — fmt fixup (gate § 3, Gate 3 FAIL).

Mechanical single-line `cargo fmt --all` run over the worktree
clears all eight whitespace-mechanical diff hunks at
`crates/ui/src/widgets/human_control.rs:202` and
`crates/ui/tests/panel_snapshots.rs:731 / :779 / :1332 / :1446 /
:1477 / :1516 / :1572`. Behaviour and test output bit-identical
afterwards. Once fmt is clean, re-run `cargo fmt --all -- --check`
to confirm exit 0, update T1916 sub-bullet's fmt acceptance with
the post-fixup output line, and route back to tester for the
second-pass verdict (Phase 4 first-pass-FAIL → second-pass-PASS
precedent applies).

The other seven gates are PASS; the fix is mechanical; **Phase 5
is one fmt run away from PASS**. `T_FINAL_<phase-5-tag>` stays
`[ ]` until tester second pass.
