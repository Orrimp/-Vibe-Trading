---
slug: v2-1-tracing-layer-redactor
mode: release
status: awaiting-operator-approval
audience: human-operator
version: 0.1.0
updated: 2026-05-29
generated: 2026-05-29T15:30:00Z
tester_commit: 0d330f6
developer_commit: c4c3cb8
---

# v2.1 tracing-Layer redactor — v0.1.0 release

## TL;DR

- A `tracing_subscriber::Layer` now redacts API keys, JWTs, AWS creds,
  and password-like fields BEFORE they hit stdout / stderr / the audit
  ledger — every future LLM call inherits this safety net for free.
- All 17 binaries (`agent`, `cockpit_live`, `backtest`, `llm-smoke`,
  10 P2 bins, …) migrated from `tracing_subscriber::fmt().init()` to a
  single `llm::tracing_init::install_global()` helper. One LoC per
  binary. Anchor delta: zero.
- Ships in WARN mode by default (logs every redaction so the operator
  can grep false-positives). 14-day observation window opens today;
  v0.2.0 flips the default to gate on 2026-06-12.

## What changed

**New files (`crates/llm/`)**

- `crates/llm/src/redact_layer.rs` — `RedactLayer` + 9-rule closed
  `RegexSet` + thread-local field-override map + custom
  `RedactingFormatFields` (substitutes redacted values during fmt
  rendering). Reuses the existing pure-fn `redact::redact()` verbatim;
  no second sanitization path. ENTROPY threshold 4.5 bits/char over
  32+ char field values (architect-ratified D-RED-1).
- `crates/llm/src/tracing_init.rs` — `install_global(extra_directives,
  json)` workspace-wide subscriber installer. Registry chain order
  locked: `EnvFilter → RedactLayer → fmt::Layer<RedactingFormatFields>`
  (R1.4 ordering contract).
- `crates/llm/tests/redact_layer.rs` — 9 integration tests (WARN /
  gate / gate+verbose modes, marker-field bypass, fail-safe-closed)
  plus 2 `#[ignore]` falsification probes (P-RED-1, P-RED-2).

**17 binaries migrated to `install_global()`**

| Tier | Binaries |
|---|---|
| **P0** (LLM-bearing) | `agent/main.rs`, `ui/src/bin/cockpit_live.rs` |
| **P1** (LLM-adjacent) | `backtest/main.rs`, `llm/src/bin/llm-smoke.rs`, `llm/src/bin/generate-replay-fixture.rs`, `trader/src/bin/llm_verdict.rs` |
| **P2** (non-LLM hygiene) | 8 × `forecast/src/bin/*.rs`, 2 × `data/src/bin/fetch_{binance,yahoo}_klines.rs`, 2 × `backtest/src/bin/{threshold_sweep,run_yahoo_sma}.rs` |

One known exclusion: `crates/ui/src/bin/cockpit.rs` retains
`tracing_subscriber::fmt().try_init()` ONLY under
`#[cfg(feature = "render-debug")]` (fixtures-mode debug path; never
LLM-bearing). Acceptable per dev Q-3; flagged for the v0.2.0 sweep.

**Workspace deps**

- `regex = "1"` added at workspace root and `crates/llm/Cargo.toml`.
- `llm` added as a peer dep of `crates/{backtest, data, forecast, ui,
  trader}` so each binary can call `install_global()`. No transitive
  new deps; `tracing-subscriber` was already a `llm` runtime dep.

## The 9-rule closed redaction set

Evaluation order is top-down; first match wins. Each match rewrites
the field VALUE via the existing `redact()` pure-fn (prefix + suffix
preserved for forensic traceability; middle scrubbed).

| # | Rule key | Pattern (regex) | Scope |
|---|---|---|---|
| 1 | `anthropic_key` | `sk-ant-[A-Za-z0-9_\-]{16,}` | value |
| 2 | `openai_proj_key` | `sk-proj-[A-Za-z0-9_\-]{16,}` | value |
| 3 | `openai_key` | `sk-[A-Za-z0-9_\-]{16,}` | value |
| 4 | `bearer_token` | `Bearer\s+[A-Za-z0-9._\-=]{20,}` | value (preserves `Bearer ` prefix) |
| 5 | `jwt` | `eyJ[A-Za-z0-9._\-]+\.eyJ[A-Za-z0-9._\-]+\.[A-Za-z0-9._\-]+` | value |
| 6 | `aws_access` | `AKIA[0-9A-Z]{16}` | value |
| 7 | `aws_secret_context` | `[A-Za-z0-9/+=]{40}` IFF field name contains `secret`/`access`/`token` | value + name context |
| 8 | `password_field_name` | n/a — exact field-NAME match: `password`/`pwd`/`passwd`/`secret`/`api_key`/`apikey`/`auth_token`/`bearer` | name |
| 9 | `entropy_fallback` | Shannon entropy ≥ 4.5 bits/char over ≥ 32 chars AND field name contains `key`/`token`/`secret` | value + name context |

The closed-set choice (Q-RED-1 (a) durable) means: adding a new
pattern requires a v0.1.x patch through analyst → architect →
developer. Operators cannot mutate the rule set at process init —
that protects against the "debug session allowlist forgotten in prod"
failure mode.

## Why

Per [`spec/v2-1-tracing-layer-redactor/feature.md § Why`](./../feature.md):
the existing pure-fn `redact()` shipped at v2-llm-strategy v2.0.0
covers only explicit `redact(secret)` call sites. A developer who
writes `tracing::info!(api_key = key, "outbound LLM request")` —
or worse, `tracing::debug!(?request, ...)` shorthand — bypasses
redaction entirely and the secret value lands in the audit ledger,
stdout, and any file appender at INFO/DEBUG. The Layer plants a
cross-cutting safety net at the `tracing_subscriber::Registry` site,
so every future LLM provider, every future audit aggregator, every
future structured-log call automatically inherits redaction with
zero per-call wiring. Pay-forward HIGH; per-cycle benefit MEDIUM;
investment SMALL (~1.5 dev days, landed on schedule).

## What you can do now

| Action | Command |
|---|---|
| Run any binary with WARN-mode redaction (default) | (no env needed — WARN is the v0.1.0 default) |
| Flip a single process to gate mode (suppress meta-events) | `REDACT_LAYER_MODE=gate <binary>` |
| Gate mode + verbose meta-events (operator diagnostics) | `REDACT_LAYER_MODE=gate REDACT_LAYER_VERBOSE=1 <binary>` |
| Grep WARN meta-events from a process's stderr log | `grep 'redact_layer' <log>` |
| Audit the rule set | `crates/llm/src/redact_layer.rs:79-110` |
| Re-run the 9 integration tests | `cargo test -p llm --test redact_layer` |
| Run the 3 falsification probes | `cargo test -p llm -- --ignored p_red_` |
| Verify anchors unchanged | `bash scripts/verify_anchors.sh` |

## Live demo — `llm-smoke --help` boots clean with `install_global()`

`llm-smoke` is a P1 LLM-bearing binary. The `--help` path exercises
clap parsing only (no real LLM call), but process init runs
`llm::tracing_init::install_global()`, which mounts `RedactLayer`
before the `fmt::Layer` per R1.4. A clean exit confirms the helper
does not panic on a fresh process boot — R-NR.5 first-line check.

```
$ cargo run -p llm --bin llm-smoke -- --help

End-to-end smoke test for the v2 LLM stack.

Usage: llm-smoke [OPTIONS]

Options:
      --mode <MODE>                Operating mode: live (real APIs), paper (record), research (replay) [default: research] [possible values: live, paper, research]
      --replay-path <REPLAY_PATH>  Override the replay cache path. Defaults to LlmConfig::default() which is `data/llm-replay.db` (live/paper) or `crates/llm/fixtures/replay-v1.db` (research, set via `--replay-path` or env override)
      --agent-toml <AGENT_TOML>    `agent.toml` path for key loading. Defaults to `config/agent.toml` (live/paper modes only; research mode skips key load) [default: config/agent.toml]
      --reset                      Delete the replay cache before opening (Q8c). Only meaningful for `--mode paper`; ignored under `live` and `research`
  -h, --help                       Print help
```

Source artifact: [`presentations/artifacts/v2-1-tracing-layer-redactor-2026-05-29/llm-smoke-help.txt`](./artifacts/v2-1-tracing-layer-redactor-2026-05-29/llm-smoke-help.txt).

And the integration suite — proof the WARN, gate, verbose, and
marker-bypass paths all run end-to-end against a real subscriber
chain:

```
$ cargo test -p llm --test redact_layer

running 11 tests
test p_red_1_layer_load_bearing ... ignored
test p_red_2_layer_ordering_documented ... ignored
test gate_mode_no_meta_event ... ok
test thread_local_peek_and_take_work ... ok
test warn_mode_records_meta_event_for_secret_field ... ok
test marker_field_bypass_with_reason_no_redaction ... ok
test password_field_name_triggers_warn_meta_event ... ok
test gate_verbose_records_meta_event ... ok
test marker_field_missing_reason_still_redacts ... ok
test only_secret_fields_are_in_redacted_fields_map ... ok
test non_secret_field_produces_no_state ... ok

test result: ok. 9 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

The 2 ignored = falsification probes (`#[ignore]` by design); they
PASS in documented state when run with `-- --ignored`.

## Screenshots

_n/a — backend tracing infrastructure feature; zero UI surface. R-NR.7
verified (zero design tokens, zero `strings.rs` adds, zero UI files
touched)._

## Verification matrix

| Gate | Command | Result | Evidence |
|---|---|---|---|
| V1 — Unit tests (`llm` lib) | `cargo test -p llm --lib` | VERIFIED — 108 pass / 0 fail / 1 ignored | test-report § 3 Gate 1 |
| V2 — Layer integration tests | `cargo test -p llm --test redact_layer` | VERIFIED — 9 pass / 0 fail / 2 ignored | live demo above + test-report § 3 Gate 2 |
| V3 — Falsification probes (P-RED-1/2/3) | `cargo test -p llm -- --ignored` | VERIFIED — all 3 PASS in documented state (load-bearing confirmed) | test-report § 8 |
| V4 — Backtest lib regression baseline | `cargo test -p backtest --lib` | VERIFIED — 45/45 pass / 5 ignored | test-report § 3 |
| V5 — R-NR.2 t1915 pure-fn parity | `cargo test -p llm redact::tests::t1915_` | VERIFIED — 6/6 pass byte-identical | test-report § 3 |
| V6 — Formatting (`fmt --all --check`) | `cargo fmt --all --check` | VERIFIED — zero diff | test-report § 2 |
| V7 — Clippy (`-D warnings` on 6 crates) | `cargo clippy -p llm -p agent -p backtest -p forecast -p data -p trader -- -D warnings` | VERIFIED — zero warnings | test-report § 2 |
| V8 — Anchor regression gate (R-NR.3) | `bash scripts/verify_anchors.sh` | VERIFIED — **84 / 84 PASS** byte-identical | live `ANCHORS PASS (84 / 84)` + test-report § 2 |
| V9 — Workspace smoke (5 builds + 3 `--help` smokes) | `cargo build -p {agent,backtest,forecast,data,ui --features live}` + `--help` on 3 binaries | VERIFIED — all builds + smokes clean; no panic on boot | test-report § 10 |
| V10 — R-NR.1 — `redact()` pure-fn unchanged | source diff | VERIFIED — `crates/llm/src/redact.rs` unmodified | test-report § 11 |
| V11 — R-NR.4 — dep-edge audit | source diff | VERIFIED — only new dep is `regex` at `llm` + workspace root | test-report § 11 |
| V12 — R-NR.5 — LLM HTTP wire untouched | binary boot smoke | VERIFIED — `llm-smoke --help` boots clean | live demo + test-report § 10 |
| V13 — R-NR.7 — zero UI / token / strings changes | source diff | VERIFIED | test-report § 11 |

## Numbers that matter

- **Tests:** 108 `llm` lib + 9 `llm` integration + 45 backtest lib +
  6 `t1915_*` pure-fn parity + 3 falsification probes = **171
  tests** green; 0 failed; 8 ignored (probes + pre-existing backtest
  ignores).
- **Anchors:** **84 / 84 PASS** byte-identical (R-NR.3). Source
  count cross-checked: `grep -c '^\[\[' spec/anchors.toml` → 84.
- **Rule set:** 9 closed rules; ENTROPY threshold 4.5 bits/char over
  32+ chars (operator-tunable at v0.2.0 if WARN window shows false
  positives).
- **Binary migration coverage:** 17 / 17 LLM-and-adjacent bins on
  `install_global()`; 1 documented exclusion (cockpit.rs fixtures
  debug path under `cfg(feature = "render-debug")`).
- **Dependency churn:** +1 production dep (`regex = "1"` — pre-audited
  BurntSushi crate, MIT/Apache, weekly downloads in the millions);
  +5 peer-crate `llm` deps (no transitive new deps).
- **Per-event runtime cost:** 9-rule regex set, O(fields) per event.
  Architect estimate at LLM-call rates (~10 req/min sustained, ~600
  redactions/hour) → negligible. No benches needed (K-arch-1
  re-evaluated at v0.2.0 if audit-write Layer lands).
- **Lines added:** ~600 LoC `redact_layer.rs` + ~70 LoC
  `tracing_init.rs` + ~250 LoC integration tests + 17 × 1-LoC
  per-binary replacement = under 1k LoC net.

## Design deviation captured for v0.2.0 ratification

**D-RED-3 (b) — "emit-redacted + filter-original" — REPLACED.** The
spec's documented pattern (re-emit the event with redacted values; a
sibling filter drops the original) cannot work in
`tracing-subscriber 0.3.x`: `tracing::warn!()` called from inside
`Layer::on_event` is silently dropped by tracing's reentrancy guard
(confirmed by empirical testing; reference at
`crates/llm/src/redact_layer.rs:23-34`). Developer M-DEV implemented
instead:

1. `RedactLayer::on_event` populates a `REDACTED_FIELDS` thread-local
   map keyed by `(field_name → redacted_value)`.
2. A custom `RedactingFormatFields` (the formatter for `fmt::Layer`)
   reads `REDACTED_FIELDS` during field rendering and substitutes
   redacted values before they reach stdout/stderr.
3. WARN-mode meta-events are written via `eprintln!` to stderr (which
   bypasses the reentrancy guard) AND recorded in a `META_EVENTS`
   thread-local so integration tests can assert on them
   programmatically.

Net effect: identical observable semantics to D-RED-3 (b). No
test-output corruption observed. Architect ratification is **deferred
to v0.2.0** alongside the planned Extensions side-channel migration
when an audit-write Layer lands. See test-report § 9 for the
INCONCLUSIVE-condition check (no corruption detected; deviation is
clean).

## Open follow-ups

1. **D-RED-3 (b) deviation** — needs architect ratification at v0.2.0
   (Extensions side-channel migration is the eventual durable shape
   when audit-write Layer lands). Tracked in test-report § 9.
2. **`cockpit.rs` render-debug exclusion** — retains
   `tracing_subscriber::fmt().try_init()` behind
   `#[cfg(feature = "render-debug")]`. Acceptable now (fixtures-mode
   debug path; never LLM-bearing). Sweep into the v0.2.0 migration
   for full coverage.
3. **14-day WARN observation window opens today (2026-05-29) →
   closes 2026-06-12.** v0.2.0 analyst at that promotion-window
   records the false-positive count from a WARN meta-event grep
   (`grep 'redact_layer.warn' <ledger>`) before flipping the default
   to `REDACT_LAYER_MODE=gate`. If the false-positive rate is
   non-zero, the analyst may also tune the entropy threshold (4.5)
   downward before flipping. Operator: monitor stderr scrollback /
   audit ledger for `redact_layer` rows during the window.

## Open decisions

_None — this is an operator-approve-to-ship release. All Q-RED-1/2/3
analyst-recommended-DURABLE options ratified by architect; no new
operator decisions surface in v0.1.0. The WARN→gate promotion
decision lands at v0.2.0 with empirical data attached._

## Approval

- [x] **Approved — ship** _(operator 2026-05-29; orthogonal lint debt accepted)_
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes / feedback

Operator approved v0.1.0 ship per Recommended path:
- D-RED-3 (b) deviation (thread-local override map vs emit-
  redacted+filter-original) accepted as v0.1.0 implementation;
  architect ratifies at v0.2.0 promotion with empirical
  WARN-observation data attached.
- cockpit.rs render-debug feature retains `tracing_subscriber::
  fmt().try_init()` — acceptable exclusion; note in v0.2.0
  migration sweep.
- 14-day WARN observation window OPENS NOW (2026-05-29 →
  2026-06-12); v0.2.0 analyst records false-positive count
  before flipping default to gate.
- spec-lint +7 debt is attributable to Bug #64 attempt-3 lane,
  NOT v2.1-redactor work — accepted as orthogonal carry-forward.

## Changelog

- 2026-05-29 (presenter): initial release-mode deck. Tester
  VERDICT → PASS at `0d330f6` (developer commit `c4c3cb8`,
  HEAD-at-test `92864cc`). 13-row verification matrix; live
  `llm-smoke --help` boot smoke + live `cargo test -p llm --test
  redact_layer` 9/9 PASS; live `bash scripts/verify_anchors.sh`
  84/84 PASS quoted. 9-rule closed redaction table + design deviation
  D-RED-3 (b) → thread-local override pattern explicitly flagged for
  v0.2.0 architect ratification. 14-day WARN observation window
  contract (2026-05-29 → 2026-06-12) recorded for v0.2.0 promotion
  pass. Sibling Pick B Wave 1 peer `ui-contrast-asserter v0.1.0`
  shipped at `68c6013`; together they form the cross-cutting safety
  duo per [`pick-b-cross-cutting-safety-duo-2026-05-29.md`](../../dev-notes/pick-b-cross-cutting-safety-duo-2026-05-29.md).
