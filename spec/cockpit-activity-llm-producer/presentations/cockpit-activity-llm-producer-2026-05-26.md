---
slug: cockpit-activity-llm-producer
version: 0.1.0
mode: release
status: draft
audience: human-operator
owner: presenter
updated: 2026-05-26
generated: 2026-05-27T07:00:00Z
predecessor: cockpit-activity-status-bar v0.1.0 (shipped 2026-05-26)
parent_forward_list: cockpit-activity-status-bar v0.1.0 § Q8 / § R5.1 (LlmCall)
verdict_source: spec/cockpit-activity-llm-producer/reports/test-final-2026-05-26-cockpit-activity-llm-producer.md
verdict_commit: a5384b4
current_commit: c46fd45
trace_row: REQ-COCKPIT-ACTIVITY-LLM-PRODUCER-001 (state = passed)
---

# cockpit-activity-llm-producer v0.1.0 — release

## TL;DR

The LLM forecaster now publishes activity events to the cockpit status bar — when the trader is reasoning, the bottom tape shows a live `LLM call: claude-3-5-sonnet-20241022` pulse instead of going silent for 45 s. PII is redacted at the producer boundary by construction (model ID only; no prompt, no symbol, no lesson cards). Zero overhead when no subscriber is attached. **Ready to ship.**

## The operator-visible win

### The "is the trader hung or thinking?" problem

Anthropic Sonnet's hard timeout is locked at **45 s** (`anthropic_impl.rs:410` Q5b architect lock). At the trader's default `fire_every_n_bars=24` cadence, an operator running a Lab backtest with `llm_forecaster_v3` enabled used to see **24-bar gaps with zero feedback** — same UX failure as the cold-cache Yahoo fetches that birthed v0.1.0 of the status bar.

That gap is now closed. Every `provider.complete()` call surfaces a Start → End row in the tape. The same 24 px bottom strip that the operator already trusts for Yahoo / Lab / Train.

### What the operator sees

When the trader fires an LLM call, the tape between the account label and the server-time label cycles:

| t (s) | Tape contents (between account · | · server-time) |
|-------|------------------------------------------------------|
| 0.0   | `[ ]` (empty)                                        |
| 0.2   | `· LLM call: claude-3-5-sonnet-20241022 · <1s`       |
| 8.0   | `· LLM call: claude-3-5-sonnet-20241022 · 8s`        |
| 12.4  | `[ ]` (Success — row removed immediately)            |

On failure (network 5xx / auth / rate limit / 45 s timeout / budget cap), the row turns red and holds for 3 s before fading — same parent R2.5 behaviour every other producer uses.

## What shipped

Single-file source edit + one new integration test file. Anchor risk **zero by construction** (the `ActivitySender` field is `Option<>` defaulted `None` in all anchored bin paths).

### 1. `ActivitySender` wired into the LLM call site

[`crates/trader/src/llm_forecaster/anthropic_impl.rs`](../../../crates/trader/src/llm_forecaster/anthropic_impl.rs) — the single hot path at the `provider.complete(request).await` site (lines 412-516 in the post-feature file). The producer:

- Creates an `ActivityHandle` via `activity_sender.start(ActivityKind::LlmCall, label)` **immediately before** the `.await`.
- Maps any `LlmForecasterError` variant to a fixed-string `handle.fail("network error" | "auth error" | "rate limited" | "server error" | "timeout 45000ms" | "budget cap" | "provider error" | ...)`.
- Explicitly `drop(activity)` **immediately after** the `.await` resolves and the failure-mapping branch runs — before `decode_response()` or `spawn_audit_row()`. The handle never crosses an `.await` boundary.
- On `Drop`, the handle emits End { Success } or End { Failed(reason) } per the parent's RAII contract — including on panic unwind (parent ADR-0042 § D2).

### 2. New `with_activity_sender()` builder setter

```rust
let forecaster = LlmForecasterImpl::new(provider, model_id, tier)
    .with_activity_sender(bus.activity()); // <- the new line
```

Both existing constructors (`new(...)` 3-arg, `with_audit_ledger(...)` 4-arg) keep their signatures **byte-identical**. The new field defaults to `None`. 153 existing trader tests + the `llm_verdict` CLI bin compile unchanged.

### 3. PII-redacted label format

```rust
const ACTIVITY_LABEL_PREFIX: &str = "LLM call: ";
// at the single format! site:
format!("{ACTIVITY_LABEL_PREFIX}{}", self.model_id)
```

The label is composed from exactly one field — `self.model_id` (a `ModelId` newtype injected at construction time, immutable per request). **No** `ForecastContext`, **no** `LlmRequest`, **no** `bar.symbol`, **no** `lesson_card` content flows into the label. Examples emitted in production:

- `LLM call: claude-3-5-sonnet-20241022`
- `LLM call: claude-3-haiku-20240307`

Tester gate T-T-2 grep-confirmed at M-FINAL: zero PII-bearing tokens (`BTCUSDT`, `price`, `prompt`, `symbol`, `lesson`) anywhere in the label-forming code path.

## Why it matters

Before v0.1.0 of the status bar shipped (2026-05-26), the operator's complaint was *"I can't tell whether the cockpit is doing something or whether it's stuck."* v0.1.0 closed that gap for Yahoo data downloads, Lab Run backtests, and the Training subprocess — but the v3 LLM forecaster was the **next biggest blind spot**: an LLM call blocks the strategy fire for up to 45 s, ten times longer than any other in-flight activity the tape was showing. v0.1.1 plugs that hole using the broadcast channel the parent built. No new architecture, no new ADR — just consumer wiring.

## What you can do now

| Action | Command |
|---|---|
| Run the live cockpit (LLM activity will surface when the trader fires) | `cargo run -p ui --bin cockpit_live --features live,yahoo` |
| Re-run the 6 new activity-tape integration tests | `cargo test -p trader --test llm_forecaster_activity_tape` |
| Re-run the full trader test suite (159 tests) | `cargo test -p trader` |
| Verify the 34 backtest anchors stayed byte-identical | `bash scripts/verify_anchors.sh` |
| Inspect the wire-up source | `crates/trader/src/llm_forecaster/anthropic_impl.rs:412-516` |

## Live demo

Replayed at the tester-PASS commit `a5384b4` on Darwin arm64 / Apple silicon, 2026-05-27.

```
$ cargo test -p trader --test llm_forecaster_activity_tape
   Compiling agent v0.1.0 (...)
   Compiling trader v0.1.0 (...)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 6.79s
     Running tests/llm_forecaster_activity_tape.rs

running 6 tests
test end_failed_event_on_llm_error ... ok
test start_event_emitted_with_correct_label_format ... ok
test activity_event_survives_cache_replay_path ... ok
test no_event_emitted_when_activity_sender_not_wired ... ok
test end_success_event_on_happy_path ... ok
test pii_redaction_label_excludes_symbol_and_prompt ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.27s
```

Interpretation:

- **start_event_emitted_with_correct_label_format** confirms R2.1 — the Start event arrives on the subscribed `broadcast::Receiver` with label exactly `"LLM call: " + model_id`.
- **end_success_event_on_happy_path** + **end_failed_event_on_llm_error** confirm R1.1 / R4.1 — the RAII drop emits Success or Failed(reason) per the parent's contract; wiremock 500 maps to `"server error"`.
- **pii_redaction_label_excludes_symbol_and_prompt** is the load-bearing K1 / H4 gate — asserts the label contains no PII-bearing field at runtime.
- **activity_event_survives_cache_replay_path** confirms the wiring composes correctly with the `BudgetedProvider` cost-cap stack.
- **no_event_emitted_when_activity_sender_not_wired** confirms R1.2 — when `with_activity_sender()` is not called, the forecaster behaves byte-identical to today (no event, no perf cost).

Raw log saved at [`presentations/artifacts/cockpit-activity-llm-producer-2026-05-26/activity-tape-tests-live-run.txt`](artifacts/cockpit-activity-llm-producer-2026-05-26/activity-tape-tests-live-run.txt).

### Anchor gate (the byte-for-byte backtest regression check)

```
$ bash scripts/verify_anchors.sh
...
ANCHORS PASS  (34 / 34)
```

Raw log saved at [`presentations/artifacts/cockpit-activity-llm-producer-2026-05-26/verify-anchors-live-run.txt`](artifacts/cockpit-activity-llm-producer-2026-05-26/verify-anchors-live-run.txt).

### Screenshot

The status-bar tape rendering is already operator-approved at the parent v0.1.0 sprint review (2026-05-26). v0.1.1 adds the `LlmCall` producer that feeds the same render path with the same accent / danger / dim colour set — no new visual surface introduced.

Reference renders (parent feature, operator-captured 2026-05-26):

- [`spec/cockpit-activity-status-bar/presentations/artifacts/cockpit-activity-status-bar-2026-05-26/`](../../cockpit-activity-status-bar/presentations/artifacts/cockpit-activity-status-bar-2026-05-26/) — parent v0.1.0 deck + manual-capture instructions for the 4 status-bar screenshots (`01-before-bare-status-bar.png` through `04-after-failure-red-row.png`).

If the operator wants a fresh capture of an `LLM call: claude-3-5-sonnet-20241022` row specifically, the manual-capture instructions in the parent deck apply verbatim — the only change is the producer kind. Sandbox is headless at presenter time; no fake screenshot has been embedded.

## Architecture call-outs

- **ADR-0042 — Cockpit activity broadcast** ([`spec/architecture/adr/0042-cockpit-activity-broadcast.md`](../../architecture/adr/0042-cockpit-activity-broadcast.md)) is the parent contract. v0.1.1 is a tactical consumer — no ADR amendment required. The `ActivityKind::LlmCall` variant was already forward-listed in § Q8; the 64-char label budget at § R1.2 is honoured (label ≤ 42 chars in production: `"LLM call: " + ≤32-char model ID`).
- **Producer-side PII redaction.** The redaction is enforced **at the producer**, not by a downstream filter. The label string is built from `self.model_id` + a module-local `const` prefix; no formatting hook can leak prompt content unless a future PR rewrites the format site. Tester grep gate T-T-2 watches for that.
- **RAII `ActivityHandle` semantic.** The handle is `!Send` (parent uses `Cell<_>` for the 100 ms tick throttle). Per the architect's M-T1 H1 falsification probe, the handle is dropped **before any subsequent `.await`** in `forecast()` — explicit `drop(activity)` precedes the sync `decode_response()`. The `!Send`-ness never crosses an `.await` boundary, so the `forecast` future remains `Send` and `async-trait`-compatible. If a future refactor inserts an `.await` between handle creation and explicit drop, the build breaks at compile time on the `async-trait`-imposed `Send` bound — H1 is structurally falsifiable.
- **Failure-state mapping.** Each `LlmForecasterError` variant maps to a fixed reason string at `handle.fail(...)`:

  | Variant | Reason string |
  |---|---|
  | `Provider(LlmError::Network(_))` | `"network error"` |
  | `Provider(LlmError::Auth(_))` | `"auth error"` |
  | `Provider(LlmError::RateLimited { .. })` | `"rate limited"` |
  | `Provider(LlmError::Provider { .. })` | `"server error"` |
  | `Provider(LlmError::BudgetExceeded { .. })` | `"budget cap"` (short-circuits before `complete()`) |
  | `Timeout { timeout_ms }` | `format!("timeout {timeout_ms}ms")` |
  | `InvalidResponse { reason }` | `format!("invalid response: {reason}")` |
  | `BudgetExceeded { .. }` (forecaster-level) | `"budget cap"` |
  | other | `"provider error"` |

  Per parent R2.5, the operator sees the **red 3 s hold** on the row; the structured reason text lives in the `ActivityEvent` for `tracing` debug but is not rendered to the operator (intentional — the row colour communicates "something failed; check logs", the structured payload is for forensics).

## Verification matrix

| Req | Status | Evidence |
|---|---|---|
| R1.1 — `ActivityHandle` wired around `provider.complete()` | VERIFIED | source: `anthropic_impl.rs:412-516`; test: `start_event_emitted_with_correct_label_format` PASS |
| R1.2 — Conditional wire-up, `Option<ActivitySender>` defaulted `None` | VERIFIED | test: `no_event_emitted_when_activity_sender_not_wired` PASS; 153 existing trader tests unchanged |
| R1.3 — No tick events (non-streaming provider) | VERIFIED | source review: only Start + End emitted, no `handle.tick(...)` calls in `forecast()` |
| R2.1 — Label = `"LLM call: " + model_id` only | VERIFIED | test: `start_event_emitted_with_correct_label_format` + tester M-FINAL T-T-2 grep gate |
| R2.2 — PII / prompt-content redaction by construction | VERIFIED | test: `pii_redaction_label_excludes_symbol_and_prompt`; T-T-2 grep audit (no PII tokens in format site) |
| R2.3 — Label ≤ 64 chars | VERIFIED | by construction (`"LLM call: "` = 10 chars; Anthropic model IDs ≤ 32 chars; total ≤ 42 chars) |
| R2.4 — Module-local `const` label prefix | VERIFIED | source: `anthropic_impl.rs:81` `const ACTIVITY_LABEL_PREFIX: &str = "LLM call: ";` |
| R3.1/R3.2 — Handle scope-dropped before next `.await` | VERIFIED | architect M-T1 H1 probe + source review at `:412-516`; future T-AR-2 audit comment in source |
| R3.4 — Drop-on-panic emits Failed | VERIFIED | parent's `ActivityHandle::drop` uses `std::thread::panicking()` check (parent ADR-0042 § D2) |
| R4.1 — All `LlmForecasterError` variants map to `handle.fail()` | VERIFIED | source: `anthropic_impl.rs:483-506` match arm; test: `end_failed_event_on_llm_error` (server error) |
| R4.2 — Red 3 s hold on failure | VERIFIED | inherited from parent R2.5 — render path unchanged |
| R4.3 — No retry inside `forecast()` | VERIFIED | source unchanged on retry policy; tester grep at M-FINAL |
| R5.1 — 34 anchors byte-identical | VERIFIED | `bash scripts/verify_anchors.sh` → `ANCHORS PASS (34 / 34)` |
| R5.2 — 153 LLM-forecaster trader tests PASS, +6 new | VERIFIED | `cargo test -p trader` → 159 passed / 0 failed / 3 ignored (pre-existing whitelist) |
| R5.3 — Cost tracking / `BudgetedProvider` / audit unchanged | VERIFIED | tested by `activity_event_survives_cache_replay_path` (BudgetedProvider stack still PASS) |
| R5.4 — No new `ActivityKind` variant | VERIFIED | source: re-uses existing `ActivityKind::LlmCall` enum value |
| R5.5 — No audit-ledger migration | VERIFIED | no SQLite schema change; tape is in-memory only |
| R5.6 — No new Lumen tokens / strings entries | VERIFIED | module-local `const` only; widget render path uses existing accent / danger / dim colours |
| R5.7 — No `crates/strategy` changes | VERIFIED | git diff shows only `crates/trader/` + `crates/trader/Cargo.toml` touched |
| R5.8 — No `crates/agent` changes | VERIFIED | no edits in `crates/agent/`; consumer-only |
| R5.9 — `cockpit-smoke` 0 panics | VERIFIED | workspace sweep `cargo test --workspace --no-fail-fast` 0 new failures |

## Numbers that matter

- **Trader test count:** **153 → 159** (delta **+6**); 0 failed; 3 pre-existing ignored.
- **6 new integration tests** in `crates/trader/tests/llm_forecaster_activity_tape.rs` (~280 LOC), all PASS in 0.27 s.
- **Source edit:** ~50 LOC in 1 file (`anthropic_impl.rs`) + ~2 lines in `Cargo.toml` (agent dep).
- **Anchor gate:** **34 / 34 PASS** — byte-identical.
- **Workspace sweep:** `cargo test --workspace --no-fail-fast` — 0 new failures.
- **Clippy:** `cargo clippy -p trader --all-targets -- -D warnings` — 0 warnings, 0 errors (34.83 s clean build).
- **Format:** `cargo fmt --check` — 0 diffs.
- **PII grep audit:** 0 PII-bearing fields (`BTCUSDT`, `price`, `prompt`, `symbol`, `lesson`) reach the label format site.
- **Label cost ceiling:** ≤ 42 chars (under the 64-char parent budget).
- **Rollback cost:** ~220 LOC, 1 source file + 1 test file, no anchors changed, no audit migration.

## Open decisions

**One — and it's a continue-or-defer call, not a code change.**

- **H3 — Is the parent's red 3 s hold + hidden reason string the right LLM-failure UX?** v0.1.1 inherits the parent's failure UX verbatim (red row, 3 s, no reason text shown). Some operators may want to see *why* an LLM call failed (`"rate limited"` vs `"timeout 45000ms"` vs `"budget cap"`) at a glance instead of digging through `tracing` logs. **Defer to operator review.** No code change required to ship; flag here so it's not lost. If the operator wants a `reason` chip in v0.1.2, that's a parent-widget change (~20 LOC in `crates/ui/src/widgets/activity_tape.rs`), not a producer change.

The three Qs from the analyst pass (Q1 label content, Q2 handle ownership, Q3 failure handling) are all closed via standing-Autoapprove at analyst-recommended defaults — no operator input pending on them.

## What's next

- **`cockpit-activity-audit-ledger` (M-DEV in flight).** The audit-ledger writer is the next producer to plug into the same broadcast channel. M-DEV is in flight at commit `c46fd45`; presenter for that lands after its M-FINAL. Same shape as this brief: builder setter, PII-redacted label, `Option<>` defaulted `None` so anchored bin paths stay byte-stable. Closes `REQ-COCKPIT-ACTIVITY-AUDIT-LEDGER-001` (currently surfaced as a `trace-broken-path` lint entry — pre-existing).
- **Live-trading loop wire-up (future brief).** Today the only `LlmForecasterImpl` constructor sites are tests + the `llm_verdict` CLI bin; neither has an `EventBus` today. When a live-trading loop wires the producer, it just calls `.with_activity_sender(bus.activity())` on the existing constructor chain. No producer code changes.
- **Per-token streaming Tick events** (v0.1.2+) — would require switching the Anthropic provider to its streaming API. Out of scope at v0.1.1.

## Sign-off

- Tester verdict: **PASS** (`spec/cockpit-activity-llm-producer/reports/test-final-2026-05-26-cockpit-activity-llm-producer.md`, run 2026-05-27 06:00 UTC, commit `a5384b4`).
- All 21 requirements (R1.1 — R5.9) VERIFIED above.
- Anchor gate: `ANCHORS PASS (34 / 34)`.
- Spec-lint: 13 new violations at M-FINAL are all from unrelated prior commits (parent `cockpit-activity-audit-ledger` trace row + `lab-polish-round-2/tasks.md` missing frontmatter); zero from this feature.

### Operator decision

- [ ] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

---

## Feedback log

_(empty — operator may append rejection notes or follow-up Qs here)_

## Changelog

- 2026-05-27 (presenter): v0.1.0 deck assembled at tester-PASS commit `a5384b4`. T-P-1 closed. Awaiting T-P-2 operator approval → ship.
