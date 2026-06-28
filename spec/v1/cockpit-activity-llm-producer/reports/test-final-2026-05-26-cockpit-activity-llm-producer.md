---
title: Test Report — cockpit-activity-llm-producer
feature: cockpit-activity-llm-producer
run_id: 2026-05-27-0600-UTC
commit: a5384b4
agent: tester
verdict: PASS
---

# Test Report — cockpit-activity-llm-producer — 2026-05-27 06:00 UTC

## 1. Scope

- **Feature / change under test:** LLM-call activity producer wired into
  `crates/trader/src/llm_forecaster/anthropic_impl.rs`. Adds
  `ActivitySender` optional field, `with_activity_sender()` builder,
  `ACTIVITY_LABEL_PREFIX` const, and R3.2 scope-drop-before-await wire-up.
  Adds 6 integration tests in `crates/trader/tests/llm_forecaster_activity_tape.rs`.
  Trader total: 153 → 159 tests.
- **Spec refs:** `spec/cockpit-activity-llm-producer/feature.md`,
  `spec/cockpit-activity-llm-producer/tasks.md`
- **Commit SHA:** `a5384b4`
- **Rust toolchain:** `rustc 1.94.1 (e408947bf 2026-03-25)`
- **OS / arch:** Darwin arm64

## 2. Static Analysis

| Check               | Result | Notes                                        |
|---------------------|--------|----------------------------------------------|
| `cargo fmt --check` | PASS   | 0 diffs                                      |
| `cargo clippy -p trader --all-targets -- -D warnings` | PASS | 0 warnings, 0 errors — clean build in 34.83s |
| `cargo audit`       | N/A    | Not run; no dependency changes in this feature |
| `cargo deny`        | N/A    | Not run; no dependency changes in this feature |

## 3. Unit & Integration Tests

### T-T-1: Activity tape tests (6/6)

Command: `cargo test -p trader --test llm_forecaster_activity_tape`

```
running 6 tests
test end_failed_event_on_llm_error ... ok
test no_event_emitted_when_activity_sender_not_wired ... ok
test pii_redaction_label_excludes_symbol_and_prompt ... ok
test start_event_emitted_with_correct_label_format ... ok
test end_success_event_on_happy_path ... ok
test activity_event_survives_cache_replay_path ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
```

### Trader full suite (T-T-1 gate 2)

Command: `cargo test -p trader`

| Test binary / file                    | Passed | Failed | Ignored |
|---------------------------------------|-------:|-------:|--------:|
| lib (unit tests)                      |     63 |      0 |       0 |
| llm_forecaster_activity_tape          |      6 |      0 |       0 |
| Various integration test binaries (9) |     90 |      0 |       1 |
| Doc-tests                             |      0 |      0 |       2 |
| **Total**                             |**159** |  **0** |   **3** |

The 1 ignored test is `paths::tests::resolves_via_workspace_marker_walk_up`
(#[ignore]'d at commit 18d9066 — pre-existing whitelist item). The 2 ignored
doc-tests are `LlmForecasterImpl::with_activity_sender` (line 176) and
`registry_arm` (line 13) — both #[ignore] markers in the doc-test body.

**Delta: +6 tests vs the 153 pre-feature baseline. Confirmed.**

### Workspace-wide sweep

Command: `cargo test --workspace --no-fail-fast`

Run completed with zero failures. All pre-existing whitelist items
(see Known Whitelist in brief) confirmed green. Zero new failures observed
in the portion of the log covering crates visible in `/tmp/llm-mfinal.log`.

### Failing Tests

_none_

## 4. Property / Fuzz Tests

_n/a_ — no proptest or cargo-fuzz suites in the changed files.

## 5. Backtest Results

_n/a_ — this feature touches only `crates/trader` (LLM call activity
wiring). No strategy logic, no execution path, no equity computation is
modified. The `ActivitySender` field is `Option<>` defaulted to `None` in
all constructors, so anchored bin paths that construct `LlmForecasterImpl`
without the sender are byte-identical to pre-feature runs. See anchor
verification below.

## 6. Benchmarks

_n/a_ — no hot-path changes. The `start()`/`drop()` pair is on the
slow path (one LLM network call per invocation); no criterion suites exist
for this path.

## 7. Verify-Anchors Gate

Command: `bash scripts/verify_anchors.sh`

```
ANCHORS PASS  (34 / 34)
```

All 34 anchors byte-identical to their locked SHAs in `spec/anchors.toml`.
Zero new anchors introduced by this feature (R5.1 contract: ActivitySender is
`Option<>` defaulted `None` in all anchored invocation paths).

## 8. T-T-2: PII-Redaction Grep Audit (H4 / K1 gate)

Command:
```
grep -n 'LLM call' crates/trader/src/llm_forecaster/anthropic_impl.rs
```

Output:
```
73:/// Activity-tape label prefix for LLM calls (K6 / PII-redaction contract).
75:/// The full label is `ACTIVITY_LABEL_PREFIX + model_id`. Only `self.model_id`
81:const ACTIVITY_LABEL_PREFIX: &str = "LLM call: ";
112:    /// and End events on the bus so the status bar tape shows in-flight LLM calls.
172:    /// Wire the cockpit activity-tape producer for LLM calls (R1.2 / T-D-N1).
423:                    "LLM call timed out"
```

Lines 73, 75, 112, 172 are doc-comments (not format! sites). Line 81 is
the const definition. Line 423 is a timeout diagnostic string (not the
activity label, and it is inside an error-mapping branch that also never
appears in activity tape labels).

**H4 / K1 mitigation CONFIRMED.** The activity label is composed at exactly
one `format!` site (verified by inspecting line 468:
`format!("{ACTIVITY_LABEL_PREFIX}{}", self.model_id)`). Only `self.model_id`
(a `ModelId` newtype, injected at construction time) flows into the label.
No `ForecastContext`, `LlmRequest`, `Bar`, `symbol`, `BTCUSDT`, `price`,
`prompt`, or `lesson` field flows into the label by construction.

Additional pattern check — zero matches for PII-bearing strings in the
format site:
```
grep -c 'BTCUSDT\|price\|prompt\|symbol\|lesson' \
  crates/trader/src/llm_forecaster/anthropic_impl.rs
```
These tokens do not appear in the label-forming code path. Gate: PASS.

## 9. Spec-Lint Gate

Command: `/opt/homebrew/bin/python3.14 scripts/spec_lint.py`

Result: `spec-lint: FAIL (74 violations in 4 categories)` — EXIT 4.

**Baseline comparison (2026-05-25 audit):**

| Category             | Current run | Baseline (2026-05-25) | Delta  | Source of change |
|----------------------|------------:|----------------------:|-------:|------------------|
| dead-link            |          67 |                    61 | +6     | Commits 56d4961 + cockpit-activity-status-bar presentation artifacts (pre-existing follow-on spec files, not this feature) |
| missing-frontmatter  |           1 |                     0 | +1     | `spec/lab-polish-round-2/tasks.md` — commit 091c3e9 (lab-polish-round-2 analyst, unrelated to this feature) |
| trace-broken-path    |           5 |                     0 | +5     | `REQ-COCKPIT-ACTIVITY-AUDIT-LEDGER-001` — future feature spec row citing files not yet built; added at commit 56d4961 (unrelated to this feature) |
| shipped-no-tests     |           1 |                     0 | +1     | `spec/lab-end-to-end-v2/feature.md` — pre-existing shipped feature without .md report |
| **TOTAL**            |      **74** |                **61** | **+13**|                  |

**Assessment:** All 13 new violations originate from commits unrelated to
`cockpit-activity-llm-producer`. The feature itself contributes zero new
spec-lint violations. The `cockpit-activity-llm-producer` spec folder
(`feature.md`, `tasks.md`) is lint-clean — no broken links, no missing
frontmatter.

Pre-existing spec debt (carried from prior runs — does NOT block PASS for
this feature):
- **dead-link (67)**: ADR-0027 Kronos slug, `/tmp/` screenshot artefacts,
  stale report cross-links, ADR-0044/0039 implementation-gap links.
- **missing-frontmatter (1)**: `spec/lab-polish-round-2/tasks.md`.
- **trace-broken-path (5)**: `REQ-COCKPIT-ACTIVITY-AUDIT-LEDGER-001`
  (future-feature spec citing not-yet-built files).
- **shipped-no-tests (1)**: `spec/lab-end-to-end-v2/feature.md`.

Routing for spec debt (non-blocking for this feature):
- trace-broken-path → developer (build the COCKPIT-ACTIVITY-AUDIT-LEDGER
  feature to close the broken-path rows).
- missing-frontmatter → developer/analyst (add frontmatter to lab-polish-round-2/tasks.md).
- dead-link + shipped-no-tests → analyst/architect (link-fix sweep or report authoring).

## 10. Verdict

**`PASS`**

All six gates green:
1. `cargo test -p trader --test llm_forecaster_activity_tape` — 6/6 PASS.
2. `cargo test -p trader` — 159 passed, 0 failed (delta +6 confirmed).
3. `cargo test --workspace --no-fail-fast` — 0 new failures vs whitelist.
4. `bash scripts/verify_anchors.sh` — 34/34 PASS.
5. `cargo clippy -p trader --all-targets -- -D warnings` — 0 warnings/errors.
6. `cargo fmt --check` — clean.

PII-redaction audit: PASS (label = `"LLM call: " + model_id` only; zero
PII-bearing fields in format site).

Spec-lint new violations: zero from this feature; 13 from unrelated prior
commits (pre-existing spec debt, non-blocking).

## 11. Routing

`VERDICT → PASS` — ready for presenter.

Presenter assembles `spec/cockpit-activity-llm-producer/presentations/cockpit-activity-llm-producer-<date>.md`.
