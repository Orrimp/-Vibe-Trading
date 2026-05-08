---
title: Test Report
feature: phase-1-foundation
run_id: 2026-05-04-2110-UTC
commit: 3efda6401e187db2a5bf9c21d83a0cbf862071f0
agent: tester
verdict: FAIL
---

# Test Report — phase-1-foundation — 2026-05-04 21:10 UTC

> **R16.3 self-check note.** Per Brief R16.3 four brand-bleed tokens
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
  fixup sub-block).
- **Rust toolchain:** `rustc 1.94.1 (e408947bf 2026-03-25)`.
- **OS / arch:** `Darwin 25.4.0 arm64`.
- **Run id:** `test-2026-05-04b-<feature-slug>` — the `b` suffix
  preserves the prior FAIL run id on disk for audit. (The prior
  report file referenced by the orchestrator,
  `spec/reports/test-2026-05-04-<feature-slug>.md`, is **not present
  on disk** at gate-run time; surfaced as an environment note in §7.)

## 2. Static Analysis

| Check | Result | Notes |
|-------|--------|-------|
| `cargo fmt --all -- --check` | PASS | exit 0, zero output |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | PASS | `Finished dev profile … in 0.71s` — zero warnings |
| `cargo audit` | N/A (not installed) | `cargo audit` command not on PATH; advisories coverage is provided by `cargo deny check` (see below — `[advisories]` table v2 in `deny.toml` resolves the same RustSec DB) |
| `cargo deny check` | PASS | `advisories ok, bans ok, licenses ok, sources ok` |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | **FAIL** | `error: public documentation for open_positions_at links to private item extract_symbol_from_description` at `crates/audit/src/query.rs:1109:24`; `error: could not document audit` |

### 2a. Doc-build failure detail (Gate 3 blocker)

```
error: public documentation for `open_positions_at` links to private item `extract_symbol_from_description`
    --> crates/audit/src/query.rs:1109:24
     |
1109 | /// existing private [`extract_symbol_from_description`] helper. Per
     |                        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ this item is private
     |
     = note: this link will resolve properly if you pass `--document-private-items`
     = note: `-D rustdoc::private-intra-doc-links` implied by `-D warnings`
     = help: to override `-D warnings` add `#[allow(rustdoc::private_intra_doc_links)]`

error: could not document `audit`
```

- **Origin:** introduced by the `real-mtm-unrealized-pnl` feature
  (commit `3efda64`), which added a `///` rustdoc link
  `[`extract_symbol_from_description`]` on the public
  `open_positions_at` fn while keeping the helper `pub(crate)`.
  Pre-dates Phase 1.
- **Why this matters for Phase 1:** the canonical `rust-validate`
  skill (`.claude/skills/rust-validate/SKILL.md` step 5) explicitly
  includes the `-D warnings` doc build. The orchestrator's T1514
  fixup sub-block claimed `rust-validate PASS` but the recorded
  command list (lines 896–904 of the task list) was fmt + clippy +
  workspace-test + verify_anchors only — **the docs step was not
  re-run**.
- **Scope of fix:** trivial — either backtick-only the symbol name
  (drop the `[ ]` link) or attach `#[allow(rustdoc::private_intra_doc_links)]`
  per the rustdoc help text. One-line developer change, audit-crate
  scope. **Anchor risk: zero** (audit/query.rs body-SHA is unaffected
  — this is a doc-comment edit only; the 11 backtest anchors live
  in audit-row content, not in public doc strings).

## 3. Unit & Integration Tests

`cargo test --workspace --all-targets` — exit 0, all suites green.
Aggregated `test result:` lines (full output captured during run):

| Crate / suite                                                           | Passed | Failed | Ignored | Duration  |
|-------------------------------------------------------------------------|-------:|-------:|--------:|----------:|
| `agent` (lib)                                                           |     44 |      0 |       0 |    0.79 s |
| `agent` (integration: bus_drops, coinbase_outage, kill_switch, …)        |     20 |      0 |       0 |   ~3.4 s  |
| `audit` (lib + integration)                                             |     63 |      0 |       0 |   ~0.4 s  |
| `backtest` (lib + integration: determinism / multi_pair / multi_symbol) |     28 |      0 |       0 |  ~46.4 s  |
| `cost` (lib)                                                            |      2 |      0 |       0 |    0.21 s |
| `data` (lib + integration)                                              |     50 |      0 |       3 |    0.6 s  |
| `exec` (lib + paper_engine_publishes)                                   |      9 |      0 |       0 |   ~0.0 s  |
| `features` (lib)                                                        |     55 |      0 |       0 |    0.17 s |
| `llm` (lib)                                                             |      0 |      0 |       0 |    0.00 s |
| `models` (lib)                                                          |      0 |      0 |       0 |    0.00 s |
| `reports` (lib + integration)                                           |    106 |      0 |       0 |  ~10.5 s  |
| `strategy` (lib)                                                        |     76 |      0 |       0 |    0.02 s |
| `trading_core` (lib + trybuild + types_test)                            |     80 |      0 |       0 |   ~3.1 s  |
| `ui` (lib)                                                              |     49 |      0 |       0 |    0.29 s |
| `ui` (integration: `panel_snapshots`)                                   | **41** |      0 |       0 |    0.31 s |
| `ui` (integration: `tape_row_click_opens_modal`)                        |  **8** |      0 |       0 |    0.00 s |
| `ui` (integration: `cockpit_live_modal_metadata_chain`)                 |      2 |      0 |       0 |    0.01 s |
| `ui` (integration: `consistency`)                                       |  **2** |      0 |       0 |    0.01 s |
| `ui` (integration: live_subscription / live_subscription_full_bus / cockpit_live_kill_button_writes_audit) | 0 / 0 / 0 default-feature; PASS under `--features live` | 0 | 0 | n/a |
| **Total (default + all-targets)**                                       | **~635** | **0** | **3** | **~66 s** |

### Spotlight tests (per Brief gate language)

| Test                                                              | Result | Output line                                                                                              |
|-------------------------------------------------------------------|--------|----------------------------------------------------------------------------------------------------------|
| `panel_snapshots` (suite)                                          | PASS   | `test result: ok. 41 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s`         |
| `tape_row_click_opens_modal` (suite)                               | PASS   | `test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s`          |
| `consistency::no_inline_user_visible_strings_in_widgets`           | PASS   | included in `test result: ok. 2 passed; 0 failed; 0 ignored …`                                            |
| `consistency::no_inline_hex_colors_in_widgets_or_state`            | PASS   | (same `consistency` suite: `test result: ok. 2 passed; 0 failed`)                                        |
| `ui::theme::tests::*` (14 tests incl. `t1501_*`, `t1503_*`, `tier_token_presence_test`, `light_palette_present`) | PASS | inside ui-lib `test result: ok. 49 passed; 0 failed`                                                      |
| `ui::widgets::frame::tests::t1505_panel_chrome_style_tokens`        | PASS   | inside ui-lib `49 passed; 0 failed`                                                                      |
| `ui::widgets::frame::tests::t1507_active_row_accent_rule`           | PASS   | inside ui-lib `49 passed; 0 failed`                                                                      |
| `panel_snapshots::status_bar_*` (4 net-new T1508 tests)             | PASS   | enumerated in panel_snapshots run output (status_bar_connected / disconnected / reconnecting / with_latency) |
| `panel_snapshots::kill_dialog_focused_input` (T1506 net-new)        | PASS   | enumerated in panel_snapshots run output                                                                 |

### Failing Tests

_none_

## 4. Property / Fuzz Tests

`proptest` is present in `core` (positive-qty test). Default-feature
run already exercises it (`prop_positive_qty_accepted` PASS in
`trading_core` lib). Larger budget run (`PROPTEST_CASES=1024`) was
not re-executed in this gate pass — the workspace is read-only-by
-tester and prior orchestrator run did not request the larger budget.

| Suite                          | Cases | Shrunk failures | Seed |
|--------------------------------|------:|----------------:|------|
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
- The prior FAIL report referenced by the orchestrator's task-list
  comment (`spec/reports/test-2026-05-04-<feature-slug>.md`) is
  **not present on disk**. This does not affect any gate but is
  surfaced for audit completeness. The `b` suffix on the present
  filename was honoured per the user's spec.
- No flaky tests observed; runtime is determinism-stable on the M-arm
  Darwin host.

## 8. Verdict

**`FAIL`**

7 of 8 gates PASS; **Gate 3 (`rust-validate`) FAILS** on the
docs-build sub-step. `RUSTDOCFLAGS="-D warnings" cargo doc --workspace
--no-deps` errors at `crates/audit/src/query.rs:1109:24` because the
public-fn rustdoc on `open_positions_at` links to the
`pub(crate)`-visibility helper `extract_symbol_from_description`,
tripping `-D rustdoc::private-intra-doc-links`. The error pre-dates
Phase 1 (introduced by the prior `real-mtm-unrealized-pnl` feature)
but the orchestrator's T1514 fixup sub-block declared
`rust-validate PASS` while only re-running fmt + clippy + workspace
-test + verify-anchors — **the docs step was skipped**. Per the
canonical rust-validate skill (step 5) and the user's gate-3
language ("rust-validate PASS via the skill — fmt, clippy …,
cargo-deny, audit, **docs**"), this is a non-negotiable failure.

The fix is one line in `crates/audit/src/query.rs:1109` (drop the
`[ ]` link, or annotate `#[allow(rustdoc::private_intra_doc_links)]`).
Anchor risk: zero. Once fixed, all 8 gates should be green and the
final tester-gate row (`T_FINAL_<feature-slug-uppercased>`) can be
ratified on the third-pass tester run.

### Gate-by-gate summary

| # | Gate | Result | Note |
|---|------|--------|------|
| 1 | Honest-tick audit (T1501–T1514 + T1514 fixup sub-block) | PASS | All ticks have file:line + test cmd + output; T1514 fixup sub-block at task-list lines 849–906 documents the 8 clippy issue groups + consistency-test cleanup. |
| 2 | `cargo test --workspace --all-targets` | PASS | All suites green (~635 passed, 0 failed, 3 ignored). `panel_snapshots` 41/41, `tape_row_click_opens_modal` 8/8, consistency 2/2, ui-lib 49/49. |
| 3 | `rust-validate` (fmt + clippy + cargo-deny + audit + docs) | **FAIL** | fmt PASS, clippy PASS, deny PASS, audit N/A (not installed; deny advisories cover it), **docs FAIL** at `crates/audit/src/query.rs:1109` (private intra-doc link). |
| 4 | `bash scripts/verify_anchors.sh` | PASS | `ANCHORS PASS  (11 / 11)` — all 11 body-SHA-256s byte-identical to `spec/anchors.toml`. |
| 5 | R16.3 brand-bleed grep on `spec/reports/` | PASS | `grep -rni …` exit 1 (zero matches). Self-check on this file: brand-bleed tokens absent (see § 0 prelude / R16.3 self-check note). |
| 6 | Cross-feature invariant table | PASS | T1512 sub-block (task-list lines 738–746) shows 7/7 PASS; tester re-ran the constituent commands and confirms each `test result: ok` line matches. |
| 7 | Snapshot baselines clean | PASS | `find crates/ui/{tests,src/widgets}/snapshots -name '*.pending-snap'` returns empty; 43 baselines on disk. |
| 8 | Visual-diff attestation | PASS | T1511 sub-block (task-list lines 631–711) records ui-designer attestation 2026-05-04 with 5 sample-attested baselines + 1 bonus + full-inventory verification + `unknown`-color sweep. |

## 9. Routing

`HANDOFF → developer` — **one-line fix in `crates/audit/src/query.rs:1109`**:
either drop the `[ ]` link around the helper-name in the rustdoc on
`open_positions_at` (rendering it as plain `\`extract_symbol_from_description\``)
or attach `#[allow(rustdoc::private_intra_doc_links)]` to the fn.
Re-run `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`
to confirm zero errors, then re-spawn tester for the third-pass
T_FINAL ratification. Anchor risk: zero (doc-comment-only edit;
audit body content unchanged).

Reference: rust-validate skill `.claude/skills/rust-validate/SKILL.md`
step 5 (`RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`).
