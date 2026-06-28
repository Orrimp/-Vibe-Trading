---
title: Test Report — Tester Verification
feature: advisor-llm-narration
run_id: 2026-06-22-1100-UTC
commit: c16a37ca507e8c8d5a37bf7598cdec819b4a3c25
agent: tester
verdict: PASS
---

# Test Report — advisor-llm-narration — 2026-06-22 11:00 UTC

## 1. Scope

- **Feature / change under test:** F9 advisor LLM "why this one" narration. Faithfulness post-check (inline narration.rs tests), anti-hallucination e2e gate, narration relay (forward_narration_relay), leaderboard narration render.
- **Spec refs:** `spec/advisor-llm-narration/feature.md`
- **Commit SHA:** `c16a37ca507e8c8d5a37bf7598cdec819b4a3c25`
- **Rust toolchain:** rustc 1.94.1 (e408947bf 2026-03-25)
- **OS / arch:** Darwin arm64

## 2. Static Analysis

| Check              | Result | Notes                                           |
|--------------------|--------|-------------------------------------------------|
| `cargo fmt --check`| PASS   | clean, exit 0                                   |
| `cargo clippy`     | PASS   | 0 warnings workspace-wide; forced re-lint via `touch crates/*/src/lib.rs` |
| `cargo audit`      | n/a    | no CVE-sensitive change in this feature         |
| `cargo deny`       | n/a    | no new deps added                               |

## 3. Unit & Integration Tests

### Narration faithfulness + anti-hallucination tests (`cargo test -p agent narration`)

| Test group | Tests | Result |
|------------|------:|--------|
| D4 faithfulness post-check (banned phrases, wrong crown, wrong outcome, fabricated numbers) | 9 | PASS |
| D11 anti-hallucination e2e (wrong crown → fellback, banned phrase → fellback, contradicted outcome → fellback, fabricated number → fellback) | 5 | PASS |
| D3 request ephemeral cache block | 1 | PASS |
| D4 faithful narration passes | 1 | PASS |
| D5 faithful fake produces ready | 1 | PASS |
| Numeric token extraction | 3 | PASS |
| Other narration tests | 1 | PASS |

**21 passed; 0 failed; 0 ignored.**

The anti-hallucination e2e tests (`d11_*`) are the day-1 non-negotiable: they prove that a narration response containing a wrong crown, a banned phrase, a contradicted outcome, or a fabricated numeric token triggers a `Fellback` state (graceful degradation to templated copy) rather than displaying a hallucinated claim to the operator.

### Narration relay (`cargo test -p ui --test forward_narration_relay`)

| Test | Result |
|------|--------|
| `explain_action_enqueues_request_with_faithful_facts` | PASS |
| `narration_outcome_relay_ready_yields_message_and_sets_state` | PASS |
| `narration_outcome_relay_fellback_yields_message_and_sets_state` | PASS |
| `forward_plan_relay_yields_message_and_populates_state` | PASS |
| `forward_plan_relay_terminates_on_sender_drop` | PASS |

**6 passed; 0 failed; 0 ignored.**

### Leaderboard narration render (`cargo test -p ui --test leaderboard_narration_render`)

| Test | Result |
|------|--------|
| `narration_not_requested_paints_explain_control` | PASS |
| `narration_fallback_paints_templated_copy_not_prose` | PASS |
| `narration_ready_paints_llm_prose_card` | PASS |
| `narration_ready_strictly_exceeds_fallback` | PASS |

**4 passed; 0 failed; 0 ignored.** Duration: 6.25 s.

### Agent crate full suite (`cargo test -p agent`)

**162 passed; 0 failed; 3 ignored** across all agent modules. The 3 ignored tests require live Anthropic API access and are correctly gated.

### Failing Tests

_none_

## 4. Property / Fuzz Tests

_n/a_ — no proptest/fuzz suites for this feature.

## 5. Backtest Results

_n/a_ — narration is a UI/agent layer; no strategy logic changed.

Anchor regression gate re-verified this session:

```
bash scripts/verify_anchors.sh
ANCHORS PASS  (119 / 119)
```

## 6. Benchmarks

_n/a_ — no latency-sensitive hot path changed.

## 7. Environment / Infrastructure Issues

The 3 ignored agent tests require a live Anthropic API key (`ANTHROPIC_API_KEY`) and are excluded from CI. The faithfulness/anti-hallucination suite uses a fake LLM response builder to avoid live API calls — fully hermetic.

## 8. Verdict

**PASS**

21 narration inline unit tests (faithfulness post-check + anti-hallucination e2e), 6 narration relay tests, and 4 leaderboard narration render tests all pass with 0 failures. The anti-hallucination e2e gate proves wrong/hallucinated narration produces `Fellback` not a displayed claim. The narration relay test confirms the faithful-facts contract (the request payload matches the `Recommendation` facts). Render tests verify the narration card paints at the pixel layer and the fallback path renders distinct templated copy. Static analysis clean workspace-wide.

## 9. Routing

`VERDICT → PASS` — ready; feature.md status bumped to `shipped`.
