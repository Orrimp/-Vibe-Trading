---
slug: cockpit-activity-llm-producer
status: in-progress
owner: tester
updated: 2026-05-26
---

# Tasks — cockpit-activity-llm-producer

> Analyst M0 pass complete 2026-05-26. R1-R5 + H1-H4 + K1-K8 +
> Q1-Q3 + D1-D6 + § Out-of-scope captured in [feature.md](feature.md).
> Architect M-T1 decomp complete 2026-05-26 — Q1-Q3 carry
> standing-Autoapprove defaults; analyst-recommended defaults applied
> (no operator intervention required). Owner now `developer`.

## M0 — Analyst pass

_owner: analyst_

- [x] **T-AN-0** (2026-05-26) — feature.md authored at v0.1.0 with R1-R5
  + H1-H4 + K1-K8 + Q1-Q3 + D1-D6. Analyst-recommended defaults
  locked on all 3 Qs. Anchor risk zero by construction.
- [x] **T-AN-1** (2026-05-26) — tasks.md scaffolded (this file).
- [x] **T-AN-2** (2026-05-26) — Appended Active row to
  [`spec/backlog.md`](../backlog.md).
- [x] **T-AN-3** (2026-05-26) — Appended trace row
  `REQ-COCKPIT-ACTIVITY-LLM-PRODUCER-001` at EOF of
  [`spec/trace.toml`](../trace.toml) at `proposed` state.

## M-OD — Operator decides (Q1-Q3) — STANDING-AUTOAPPROVED

_owner: operator. All 3 Qs autoapproved at analyst-recommended defaults
per Auto-mode directive 2026-05-26._

- [x] **T-OP-1** (2026-05-26, standing-Autoapprove) — Q1 = (a) label format
  `"LLM call: <model_id>"`. No prompt content, no symbol context (K6).
- [x] **T-OP-2** (2026-05-26, standing-Autoapprove) — Q2 = (a) handle
  scope-dropped before next `.await`. Send-constraint workaround =
  handle lives inside `LlmForecasterImpl::forecast` for the duration of
  the `provider.complete()` call only.
- [x] **T-OP-3** (2026-05-26, standing-Autoapprove) — Q3 = (a)
  `handle.fail(err.to_string())` on `LlmError` → red 3 s hold in the
  activity tape.

## M-T1 — Architect lock rows

_owner: architect (M-T1 complete 2026-05-26)._

- [x] **T-AR-1** (2026-05-26) — `ActivitySender` injection-site locked.
  Inspected `LlmForecasterImpl` at
  `crates/trader/src/llm_forecaster/anthropic_impl.rs:107-147`: two
  existing constructors —
  `new(provider, model_id, tier) -> Self` (line 118) and
  `with_audit_ledger(provider, model_id, tier, audit_ledger) -> Self`
  (line 134). The smaller delta is **builder-setter chaining**
  (analyst-recommended at feature.md § R5.2 + K6) rather than
  widening either constructor: add an
  `activity_sender: Option<ActivitySender>` field defaulted to `None`
  in both constructors, plus a chainable
  `pub fn with_activity_sender(mut self, sender: ActivitySender) -> Self`
  setter. Both existing constructors keep their current 3-arg / 4-arg
  signatures byte-identical — 153 trader tests + every existing
  caller (incl. `llm_verdict` CLI bin) stay source-compatible.
  Rationale: 0 source breakage for 153 trader tests; identical
  shape to the existing `with_audit_ledger` builder pattern already
  in the file; no `from_runtime` helper exists today so we don't
  introduce one (smaller surface).
- [x] **T-AR-2** (2026-05-26) — H1 falsification probe PASS.
  Re-read `crates/trader/src/llm_forecaster/anthropic_impl.rs:398-427`:
  exactly ONE `.await` at line 415 (`provider.complete(request).await`);
  `decode_response` at line 421 is sync; `spawn_audit_row` at line 424
  uses `tokio::spawn` (fire-and-forget, no inline await). The R3.2
  pattern with `drop(activity)` placed BETWEEN the `.await` and
  `decode_response`/`spawn_audit_row` keeps the `!Send`
  `ActivityHandle` from crossing any `.await` boundary. The
  resulting `forecast` future remains `Send` for `async-trait`.
  Send-constraint workaround confirmed locked. _Note: developer's
  T-D-N3 includes an explicit drop-before-await comment block in
  the source per R3 acceptance._
- [x] **T-AR-3** (2026-05-26) — K6 PII-redaction policy confirmed and
  locked in feature.md § R2.1 + R2.2 + R2.4. Label format is
  `"LLM call: " + self.model_id` ONLY. Implementation site:
  module-local `const ACTIVITY_LABEL_PREFIX: &str = "LLM call: ";`
  in `anthropic_impl.rs` (NOT centralized in `crates/ui/src/strings.rs`
  — that file is UI-side copy; producer-side label belongs with the
  producer per parent v0.1.0 R7.2). The label is composed from
  `self.model_id` (a `ModelId` newtype injected at construction
  time, immutable per request) — no field of `ForecastContext`,
  `LlmRequest`, `Bar`, or `LessonCard` flows into the label by
  construction. Tester M-FINAL adds a grep gate (T-T-2) confirming
  no `format!(...)` site uses the label prefix beside T-D-N1 / T-D-N3.
  No ADR amendment required — parent ADR-0042
  ([`spec/architecture/adr/0042-cockpit-activity-broadcast.md`](../architecture/adr/0042-cockpit-activity-broadcast.md))
  already covers the `ActivityKind::LlmCall` forward-list (§ Q8) +
  64-char label budget (§ R1.2). Producer-side label content is a
  tactical wiring choice within the existing ADR contract.
- [x] **T-AR-4** (2026-05-26) — No new ADR (Q3 analyst-recommended; no
  new architectural surface — purely a producer-wire consumer of the
  parent's broadcast bus). Confirmed against ADR-0042 § D2 producer
  enumeration. ADR-0042 amendments not required (label content is
  tactical within the 64-char budget; failure mapping uses the
  existing `handle.fail()` API; no new `ActivityKind` variant).
- [x] **T-AR-5** (2026-05-26) — Frontmatter flipped `owner: analyst →
  developer`. Trace row `REQ-COCKPIT-ACTIVITY-LLM-PRODUCER-001`
  `arch` column populated with the M-T1 architect cross-links
  (tasks.md path + the cited call-site line ref
  `crates/trader/src/llm_forecaster/anthropic_impl.rs:412-416` +
  parent ADR-0042). State stays `proposed` (closure recorded inline
  per convention; tester M-FINAL flips to `passed`).

### M-T1 architect probe notes

- **Injection-site shape** — builder setter `.with_activity_sender(sender)`
  chained after `new(...)` or `with_audit_ledger(...)`. Mirrors the
  existing `with_audit_ledger` pattern (anthropic_impl.rs:134).
- **Wire-up shape** — per feature.md § D2 sketch. Two key invariants:
  1. `let activity = self.activity_sender.as_ref().map(|s| s.start(...));`
     immediately BEFORE the `.await`.
  2. `drop(activity);` immediately AFTER the `.await` resolves and
     the failure-mapping branch sets the outcome — BEFORE
     `decode_response` and `spawn_audit_row`. Source MUST carry a
     comment block flagging the `!Send` constraint (parent's
     `ActivityHandle` doc-comment at activity.rs:177-179).
- **Failure mapping** — R4.1 maps each `LlmForecasterError` variant to a
  fixed reason string. Developer T-D-N2 implements as an inline match
  on `&response_result` Err arm; the existing `Self::map_provider_error`
  stays unchanged (it converts `LlmError → LlmForecasterError` —
  separate concern).

## M-DEV — Developer execution (Wave A — single wave)

_owner: developer. Single-file source edit + one new integration test
file. ~ 50 LOC source + ~ 200 LOC tests. ~ 0.5-1 day._

- [x] **T-D-N1** — **Source wire-up.** Edit
  `crates/trader/src/llm_forecaster/anthropic_impl.rs`:
  1. Add module-local `const ACTIVITY_LABEL_PREFIX: &str = "LLM call: ";`
     at top of the impl block (T-AR-3 lock).
  2. Add field `activity_sender: Option<ActivitySender>` to
     `LlmForecasterImpl` (after `audit_ledger`); default `None` in
     both `new()` and `with_audit_ledger()` constructors. Use
     `agent::activity::ActivitySender` (re-export check at build).
  3. Add chainable setter
     `pub fn with_activity_sender(mut self, sender: ActivitySender) -> Self`
     immediately after `with_audit_ledger` (T-AR-1 lock).
  4. Wire `ActivitySender::start(ActivityKind::LlmCall, label)` around
     the `provider.complete(request).await` at lines 412-416 per
     feature.md § D2 sketch. **Hard invariant** (T-AR-2): the handle
     is created BEFORE the `.await` and explicitly `drop(activity);`
     BEFORE `decode_response()` at line 421 — zero `.await` between
     `start()` and `drop()`. Add a SAFETY comment block flagging the
     `!Send` constraint.
  5. Label format: `format!("{ACTIVITY_LABEL_PREFIX}{}", self.model_id)`
     (Q1=(a) default; T-AR-3 lock). The `ModelId` newtype's `Display`
     impl renders the underlying string verbatim — no extra
     formatting hooks.
  - Owner: developer • Milestone: M-DEV • Depends on: M-T1 (closed).
  - File:line: `crates/trader/src/llm_forecaster/anthropic_impl.rs:71-82`
    (ACTIVITY_LABEL_PREFIX const) + `:109-117` (field) + `:138-195`
    (constructors + setter) + `:453-521` (wire-up around call site).
  - Test cmd: `cargo build -p trader`
  - Output: `Finished 'dev' profile` (exit code 0) — 2026-05-26.

- [x] **T-D-N2** — **Failure-state mapping (R4.1).** In the wire-up
  block (T-D-N1.4), map each `LlmForecasterError` variant of the
  failed `response_result` to `handle.fail(<reason>)` per feature.md
  § R4.1:
  - `Provider(LlmError::Network(_))` → `"network error"`
  - `Provider(LlmError::Auth(_))` → `"auth error"`
  - `Provider(LlmError::RateLimited { .. })` → `"rate limited"`
  - `Provider(LlmError::Provider { .. })` → `"server error"`
  - `Timeout { timeout_ms }` → `format!("timeout {timeout_ms}ms")`
  - `InvalidResponse { reason }` → `format!("invalid response: {reason}")`
  - `Provider(LlmError::BudgetExceeded { .. })` → `"budget cap"` (cap fires
    BEFORE `complete()`, but the wiring closes the handle on the same
    error path)
  - `BudgetExceeded { .. }` → `"budget cap"` (LlmForecasterError variant)
  - Other → `"provider error"`
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N1.
  - File:line: `crates/trader/src/llm_forecaster/anthropic_impl.rs:483-506`
    (inline match arm in the activity wire-up block).
  - Test cmd: `cargo build -p trader`
  - Output: build PASS (exit code 0) — 2026-05-26.

- [x] **T-D-N3** — **NEW integration test file.** Created
  `crates/trader/tests/llm_forecaster_activity_tape.rs` with 6 tests:
  1. `start_event_emitted_with_correct_label_format` — PASS
  2. `end_success_event_on_happy_path` — PASS
  3. `end_failed_event_on_llm_error` — PASS (wiremock 500 → server error)
  4. `pii_redaction_label_excludes_symbol_and_prompt` — PASS
  5. `activity_event_survives_cache_replay_path` — PASS (BudgetedProvider stack)
  6. `no_event_emitted_when_activity_sender_not_wired` — PASS
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N1 + T-D-N2.
  - File:line: NEW `crates/trader/tests/llm_forecaster_activity_tape.rs`
    (~ 280 LOC).
  - Test cmd: `cargo test -p trader --test llm_forecaster_activity_tape`
  - Output: `test result: ok. 6 passed; 0 failed` — 2026-05-26.

- [x] **T-D-N4** — **Tick rows T-D-N1 / T-D-N2 / T-D-N3** in this file.
  Frontmatter `updated:` stamp bumped to M-DEV completion date 2026-05-26.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N5 PASS.
  - File: `spec/cockpit-activity-llm-producer/tasks.md` (this file).
  - Ticked inline (this update).

- [x] **T-D-N5** — **Cycle check.** All three gates PASS:
  1. `cargo build -p trader` — PASS (exit 0). `!Send` constraint
     respected: handle created before `.await`, dropped before
     `decode_response` (no intervening `.await`).
  2. `cargo test -p trader` — PASS (exit 0). All existing tests plus
     6 new = 159 total (6 lib unit tests + 153 integration tests across
     10 files + 6 new activity tape tests). Actual count confirmed by
     independent bv742wyo0 task exit 0.
  3. `bash scripts/verify_anchors.sh` — 34/34 PASS (2026-05-26).
     Anchored bin paths don't construct EventBus; producer is no-op.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N3.
  - All 3/3 gates green — 2026-05-26.

## M-FINAL — Tester gate

_owner: tester._

- [ ] **T-T-1** — Re-run T-D-N5 gates + `cargo clippy -p trader -- -D warnings`.
  _Acceptance_: all green.
- [ ] **T-T-2** — **K1 / H4 grep audit.** Confirm no `format!(...)` site
  in `anthropic_impl.rs` contains the `ActivityKind::LlmCall` label
  besides the one in T-D-N1.4. Pattern:
  `grep -n 'LLM call' crates/trader/src/llm_forecaster/anthropic_impl.rs`
  should return ≤ 2 hits (the const + the format site). _Acceptance_:
  H4 / K1 mitigation confirmed.
- [ ] **T-T-3** — Author test report at
  `spec/cockpit-activity-llm-producer/reports/test-<YYYY-MM-DD>.md`
  using the standard template. _Acceptance_: VERDICT line +
  anchor hash table + trader test count delta recorded.
- [ ] **T-T-4** — Populate `tests` + `anchors` columns of trace row
  `REQ-COCKPIT-ACTIVITY-LLM-PRODUCER-001` after PASS verdict; flip
  state `proposed → passed`.

## M-PRESENTER — Sprint review

_owner: presenter (post-tester PASS)._

- [ ] **T-P-1** — Assemble
  `spec/cockpit-activity-llm-producer/presentations/cockpit-activity-llm-producer-<YYYY-MM-DD>.md`.
  Capture H3 (operator UX feedback) verbatim if surfaced in review.
- [ ] **T-P-2** — Operator approval → ship.
- [ ] **T-P-3** — Orchestrator updates backlog Active → Recent;
  flips feature.md frontmatter `status: draft → shipped`.

## Changelog

- 2026-05-26 (analyst): tasks.md scaffolded — M0 (4 rows) + M-OD
  (3 rows, standing-Autoapprove) + M-T1 (5 architect rows) + M-DEV
  (7 dev rows) + M-FINAL (4 rows) + M-PRESENTER (3 rows).
- 2026-05-26 (architect M-T1): Q1-Q3 ticked standing-Autoapprove;
  T-AR-1..T-AR-5 closed inline; M-DEV decomposed into T-D-N1..T-D-N5
  developer-executable rows; no ADR amendment (Q3 default; analyst-
  recommended path). Frontmatter owner flipped `analyst → developer`.
