---
title: Test Report
feature: phase-1-foundation
run_id: 2026-05-04c-2200-UTC
commit: 3efda6401e187db2a5bf9c21d83a0cbf862071f0
agent: tester
verdict: PASS
---

# Test Report — phase-1-foundation — 2026-05-04 22:00 UTC (third pass)

> **R16.3 self-check note.** Per Brief R16.3, four brand-bleed tokens
> (the design-system name plus the three tier/elevation tokens listed
> in the user's gate-5 grep pattern) must not appear in
> `spec/reports/` bodies. This report deliberately elides those four
> literals in prose. Task-list and feature-brief paths are referred
> to by the placeholder `<feature-slug>` wherever the literal slug
> would otherwise leak into report content. The grep pattern itself
> is reproduced ONLY in § 8 / § 9 routing context where the brief
> requires verbatim citation, and is split across line breaks so the
> contiguous regex never appears.

## 1. Scope

- **Feature / change under test:** Phase 1 Foundation — design-system
  adoption (token rewrite, 3-tier elevation, whisper-shadow ladder,
  focus ring, active-row pattern, status_bar widget, principles-doc
  supersede, 36-snapshot refresh).
- **Spec refs:** `spec/features/<feature-slug>.md`,
  `spec/tasks/<feature-slug>.md`.
- **Commit SHA:** `3efda6401e187db2a5bf9c21d83a0cbf862071f0` (worktree
  has uncommitted Phase 1 edits per task-list T1501–T1514 + T1514
  fixup sub-block + T1514 rustdoc gate addendum).
- **Rust toolchain:** `rustc 1.94.1 (e408947bf 2026-03-25)`.
- **OS / arch:** `Darwin 25.4.0 arm64`.
- **Run id:** `test-2026-05-04c-<feature-slug>` — the `c` suffix
  preserves both prior FAIL run ids on disk for audit
  (`test-2026-05-04-<feature-slug>.md` first-pass FAIL: fmt + clippy;
  `test-2026-05-04b-<feature-slug>.md` second-pass FAIL: rustdoc gate).

## 2. Static Analysis

| Check | Result | Notes |
|-------|--------|-------|
| `cargo fmt --all -- --check` | PASS | exit 0, zero output |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | PASS | `Finished dev profile … in 1.36s` — zero warnings |
| `cargo audit` | N/A (not installed) | `cargo audit` command not on PATH; advisories coverage is provided by `cargo deny check` (`[advisories]` table v2 in `deny.toml` resolves the same RustSec DB) — same handling as the second-pass report |
| `cargo deny check` | PASS | `advisories ok, bans ok, licenses ok, sources ok` |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | **PASS** | `Finished dev profile … in 14.40s`; `Generated … target/doc/agent/index.html and 15 other files`. Zero errors. |

### 2a. Doc-build resolution detail (Gate 3 closure)

The second-pass tester run flagged three `rustdoc::private_intra_doc_links`
errors that pre-dated Phase 1 (introduced by the prior
`real-mtm-unrealized-pnl` and `v1-5b-multi-venue` features).
Orchestrator-applied doc-comment-only edits at:

- `crates/audit/src/query.rs:1109` — `[`extract_symbol_from_description`]`
  → `` `extract_symbol_from_description` ``.
- `crates/agent/src/runtime.rs:80` — `[`spawn_feed_taps_with_observer`]`
  → `` `spawn_feed_taps_with_observer` ``.
- `crates/agent/src/runtime.rs:708` — `[`spawn_feed_taps`]`
  → `` `spawn_feed_taps` ``.

Re-ran `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`
→ exit 0; clean build. Anchor risk: zero (doc-comment-only edits;
audit-row body content unchanged → backtest body-SHA-256s
preserved; verified via Gate 4 below).

## 3. Unit & Integration Tests

`cargo test --workspace --all-targets` — exit 0, all suites green.

| Metric | Value |
|---|---|
| Test binaries run | 96 |
| Tests passed | **757** |
| Tests failed | **0** |
| Tests ignored | 3 |
| Failing tests | _none_ |

### Spotlight tests (per Brief gate language)

| Test | Result | Output line |
|------|--------|-------------|
| `panel_snapshots` (suite) | PASS | `test result: ok. 41 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s` |
| `tape_row_click_opens_modal` (suite) | PASS | `test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s` |
| `consistency::no_inline_user_visible_strings_in_widgets` | PASS | included in `test result: ok. 2 passed; 0 failed; 0 ignored …` |
| `consistency::no_inline_hex_colors_in_widgets_or_state` | PASS | (same `consistency` suite: `test result: ok. 2 passed; 0 failed`) |
| `ui::theme::tests::*` (incl. T1501 / T1503 / tier-token / light-palette pins) | PASS | inside ui-lib `49 passed; 0 failed` |
| `ui::widgets::frame::tests::t1505_panel_chrome_style_tokens` | PASS | inside ui-lib `49 passed; 0 failed` |
| `ui::widgets::frame::tests::t1507_active_row_accent_rule` | PASS | inside ui-lib `49 passed; 0 failed` |
| `panel_snapshots::status_bar_*` (4 net-new T1508 tests) | PASS | enumerated in panel_snapshots run output (`status_bar_connected` / `status_bar_disconnected` / `status_bar_reconnecting` / `status_bar_with_latency`) |
| `panel_snapshots::kill_dialog_focused_input` (T1506 net-new) | PASS | enumerated in panel_snapshots run output |

### Failing Tests

_none_

## 4. Property / Fuzz Tests

`proptest` is present in `core` (positive-qty test). Default-feature
run already exercises it (`prop_positive_qty_accepted` PASS in
`trading_core` lib). Larger budget run (`PROPTEST_CASES=1024`) was
not re-executed in this gate pass — the workspace is read-only-by-
tester and prior orchestrator runs did not request the larger budget.

| Suite | Cases | Shrunk failures | Seed |
|---|---:|---:|---|
| `core::prop_positive_qty_accepted` (default budget) | 256 | 0 | system-default |

## 5. Backtest Results

_n/a — Phase 1 Foundation is UI-only. No `crates/strategy/`,
`crates/audit/` (logic), `crates/exec/`, `crates/backtest/`, or
`crates/reports/` rendering code touched. The 11 body-SHA-256
backtest anchors are verified byte-identical via Gate 4 below._

## 6. Benchmarks

_n/a — Phase 1 changes are token / chrome / widget-shape only; no
hot-path code touched (no order-book, feature-calc, or inference
changes)._

## 7. Environment / Infrastructure Issues

- `cargo audit` not installed on PATH. Per the rust-validate skill:
  "Install with `cargo install cargo-audit` if missing; ask the user
  before installing." Tester role does not auto-install. Coverage
  gap is bridged by `cargo deny check` (`[advisories]` v2 against
  the same RustSec DB) which PASSES.
- The prior FAIL reports referenced by the orchestrator's task-list
  comments (`spec/reports/test-2026-05-04-<feature-slug>.md` and
  `…b-<feature-slug>.md`) are present on disk; both preserved for
  audit per the user's `<run-id>{,b,c}` suffix scheme.
- No flaky tests observed; runtime is determinism-stable on the
  M-arm Darwin host.

## 8. Verdict

**`PASS`**

All 8 gates PASS.

### Gate-by-gate summary

| # | Gate | Result | Note |
|---|------|--------|------|
| 1 | Honest-tick audit (T1501–T1514 + T1514 fixup sub-block + T1514 rustdoc gate addendum) | PASS | All ticks have file:line + test cmd + output. T1514 fixup sub-block at task-list lines 849–924 documents the original 8 clippy issue groups + consistency-test cleanup + the new rustdoc gate fix (3 doc-link edits in audit/query.rs + agent/runtime.rs). |
| 2 | `cargo test --workspace --all-targets` | PASS | All suites green: 757 passed, 0 failed, 3 ignored across 96 test binaries. `panel_snapshots` 41/41, `tape_row_click_opens_modal` 8/8, consistency 2/2, ui-lib 49/49. |
| 3 | `rust-validate` (fmt + clippy + cargo-deny + audit + docs) | PASS | fmt PASS, clippy PASS, deny PASS, audit N/A (not installed; deny advisories cover it), docs PASS. The second-pass blocker (private intra-doc link in audit/query.rs:1109) is closed; two more pre-Phase-1 sites in agent/runtime.rs (lines 80, 708) cleared the same way. |
| 4 | `bash scripts/verify_anchors.sh` | PASS | `ANCHORS PASS  (11 / 11)` — all 11 body-SHA-256s byte-identical to `spec/anchors.toml`. Confirms doc-comment-only edits did not perturb audit-row content. |
| 5 | R16.3 brand-bleed grep on `spec/reports/` | PASS | `grep -rni …` exit 1 (zero matches). Self-check on this file: brand-bleed tokens absent (see prelude / R16.3 self-check note). |
| 6 | Cross-feature invariant table | PASS | T1512 sub-block (task-list lines 715–749) shows 7/7 PASS for v1.5b multi-venue, kill-switch, journal modal, latency badge, market health, kill confirmation, modal flow. Tester re-ran via the umbrella workspace test. |
| 7 | Snapshot baselines clean | PASS | `find crates/ui/tests/snapshots -name '*.pending-snap'` returns empty; 41 baselines on disk (36 refreshed + 5 net-new for T1506/T1508). |
| 8 | Visual-diff attestation | PASS | T1511 sub-block (task-list lines 607–714) records ui-designer attestation 2026-05-04 with sample-attested baselines + full-inventory verification + `unknown`-color sweep. |

## 9. Routing

`VERDICT → PASS` — handoff to **presenter** for the Phase 1
sprint-review deck.

The presenter spawn must run the canonical `scripts/check_presentation.sh`
mechanical pre-tick gate before READY, capture both bin screenshots
(`cargo run --bin cockpit --features fixtures` and
`cargo run --bin cockpit_live --features live -- --config config/agent.toml`),
and assemble `spec/presentations/<feature-slug>-2026-05-04.md` for
operator approval. Phase 2 (Backtest panel) is queued and gated on
this presentation accepted by the operator.

Reference: rust-validate skill `.claude/skills/rust-validate/SKILL.md`
step 5 (`RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`)
— now PASS for the first time in the Phase 1 tester sequence.
