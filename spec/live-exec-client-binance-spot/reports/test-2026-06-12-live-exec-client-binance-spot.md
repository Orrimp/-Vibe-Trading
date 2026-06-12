---
title: Test Report — F1 live-exec-client-binance-spot close-out gate
feature: live-exec-client-binance-spot
run_id: 2026-06-12-1800-UTC
commit: dc3ef58910a39c58a808fbdea26b0c10578518c9
agent: tester
verdict: PASS
---

# Test Report — live-exec-client-binance-spot — 2026-06-12 18:00 UTC

## 1. Scope

- **Feature / change under test:** F1 — Binance Spot live execution client + real-exchange reconciliation (TESTNET-FIRST). Authenticated `BinanceSpotExecClient` behind `LiveExecRouter` + `AccountReader` traits; `SecretSource` trait + `SecretString` redaction; exchange-filter pre-validation; two-class reconciliation loop (SOFT debounce N=2 + HARD immediate); exec-side cap mechanism; HMAC signing + clock-skew handling; error/retry/idempotency taxonomy; `#[ignore]`-gated testnet rehearsal suite.
- **Spec refs:** `spec/live-exec-client-binance-spot/feature.md`, `spec/live-exec-client-binance-spot/tasks.md`, `spec/architecture/adr/0054-mode-live-boundary.md`
- **Commit SHA:** `dc3ef58910a39c58a808fbdea26b0c10578518c9`
- **Rust toolchain:** `rustc 1.94.1 (e408947bf 2026-03-25)`
- **OS / arch:** `Darwin 25.5.0 arm64`
- **Developer claim:** waves A–F1 complete; all tests green; clippy/fmt/deny clean; 119/119 anchors; spec-lint 70 (zero-new)

---

## 2. Static Analysis

| Check | Result | Notes |
|---|---|---|
| `cargo fmt --check -p exec -p agent -p trading_core` | **PASS** | Zero diffs on all touched crates |
| `cargo clippy -p exec -p agent -p trading_core` | **PASS** | Zero warnings attributable to new lines |
| `cargo deny check advisories` | **PASS (pre-existing)** | One pre-existing advisory: `paste` unmaintained. No new advisories from hmac/hex additions. |
| `cargo deny check licenses` | **FAIL (pre-existing only)** | `polars-arrow-format` license not specified — pre-existing baseline, not introduced by F1. No new license violations. |
| `cargo deny check bans` | **PASS** | No new banned crates |
| `cargo deny check sources` | **PASS** | No new unknown sources |

**Dependency gate verdict:** No new security findings or ban violations from F1's `hmac`/`hex` additions. The `paste` unmaintained advisory and `polars-arrow-format` license issue are pre-existing (present before F1, unchanged).

---

## 3. Unit & Integration Tests

### 3a. Suite Counts

| Crate / Suite | Passed | Failed | Ignored |
|---|---:|---:|---:|
| `trading_core` (lib + compile-fail) | 100 | 0 | 0 |
| `exec` (lib + all integration tests incl. testnet skip) | 38 | 0 | 3 |
| `agent` (lib + all integration tests) | 132 | 0 | 2 |
| `audit` (lib) | 187 | 0 | 1 |
| `ui --lib` (collateral proof) | 447 | 0 | 0 |
| **TOTAL** | **904** | **0** | **6** |

Notes:
- The 3 ignored in `exec` are `binance_testnet_live.rs::place_order_testnet`, `account_read_testnet`, `reconcile_no_divergence_testnet` — operator-gated (AC-13 / M-DEV-F1). Skip message: `"operator-only: requires BINANCE_EXEC_LIVE_TESTNET=1 + testnet keys"`.
- The 2 ignored in `agent` are doc-test stubs (pre-existing).
- The 1 ignored in `audit` is a doc-test stub (pre-existing).

### 3b. Named F1 Tests — Complete Inventory

**`crates/exec --test live_exec_adversarial` (7 tests)**
- `live_exec_router_trait_exists` — AC-1: trait shape (place/status/cancel via FakeTransport)
- `account_reader_parses_decimal` — AC-4: `AccountReader` parses `Decimal` from recorded fixture
- `order_observably_submitted_once` — AC-7 (adversarial): exactly-1 submission, newClientOrderId round-trip
- `ambiguous_timeout_queries_before_resubmit` — AC-8 (adversarial): transport error → status query before retry
- `decimal_only_compile_time_guard` — AC-9: compile-time float_arithmetic deny + Decimal type check
- `no_real_exchange_no_real_key_in_ci` — AC-12 (adversarial): keys unset, Network::default() == Testnet
- `cap_exceeded_fake_transport_receives_zero_requests` — AC-11 (adversarial): integration-level cap check

**`crates/exec --test binance_testnet_live` (3 ignored)**
- `place_order_testnet` — AC-13 F2-gate (operator-only)
- `account_read_testnet` — AC-13 F2-gate (operator-only)
- `reconcile_no_divergence_testnet` — AC-13 F2-gate (operator-only)

**`crates/agent --test live_reconcile_adversarial` (7 tests) — AC-10**
- `soft_once_then_clear_no_halt` — SOFT single read does not trip (debounce proven)
- `reconcile_divergence_trips_halt` — SOFT N=2 consecutive reads → `LedgerImbalance` trip (asserted via `ks_clone.is_tripped()`)
- `reconcile_unknown_position_hard_trips` — HARD unknown-asset position → immediate trip, first read
- `hard_immediate_trips_on_first_read` — HARD exchange-only BTC position → immediate trip
- `tolerance_boundary_exact_no_halt` — tolerance-boundary-exact: delta == tolerance → NOT tripped (boundary inclusive on the allowed side)
- `paper_mode_reconcile_is_noop` — `AccountReader = None` → no halt (paper path unchanged)
- `soft_divergence_counter_resets_on_clear` — counter resets to 0 on in-tolerance read; subsequent diverge starts fresh N=2 cycle

**`crates/core/src/secret.rs` (inline tests)**
- `secret_never_logged_or_serialized` — AC-2: Debug/Display emit `"<redacted>"`, serde refused
- `has_proxies_get` — SecretSource::has() mirrors get()

**`crates/agent/src/secret.rs` (inline tests)**
- `missing_secret_fails_closed_env` — AC-3: absent env var → `Missing`, never default
- `empty_env_var_is_missing` — AC-3: empty env var → `Missing`
- `missing_secret_fails_closed_local_file` — AC-3: nonexistent file → `Missing`
- `local_file_reads_value` — reads from temp TOML file, asserts fake placeholder strings

**`crates/exec/src/live/` (inline unit tests — selected)**
- `default_endpoint_is_testnet` (endpoint.rs) — AC-12/AQ-6: Network::default() == Testnet, label == "testnet", base_url != "api.binance.com"
- `testnet_and_mainnet_are_distinct` (endpoint.rs) — distinct URLs, distinct labels
- `signer_reproduces_fixed_vector` (sign.rs) — AC-6: HMAC-SHA256 against Binance docs public example vector
- `signer_fake_key_vector` (sign.rs) — AC-6: 64-char hex output, self-consistent
- `binance_code_maps_to_variant` (error.rs) — retry taxonomy: -1003/-1021/-1022/-2010/-2014/-2015/-1013 → correct variant
- `exec_side_cap_rejects_over_notional` (cap.rs) — AC-11: boundary==cap ALLOWED, >cap REJECTED (5 parametrized cases)
- `cap_exceeded_error_carries_values` (cap.rs) — error carries exact notional+cap values
- `clock_skew_resyncs_then_halts` (clock.rs) — AC-6/R5: -1021 triggers resync, persistent skew → ClockSkew
- `under_min_notional_fails_fast` (filters.rs) — AC-5 (adversarial): zero network requests
- `bad_lot_step_rejected` (filters.rs) — AC-5 (adversarial): stepSize violation → FilterReject
- `valid_order_passes` / `round_to_step_non_trivial` / `parse_filters_from_json_btcusdt` / `filter_cache_ttl` (filters.rs)

### Failing Tests

_none_ — 0 failures across all suites.

---

## 4. AC Verification Matrix (all 15 ACs)

| AC | Description | Test Citation | Result |
|---|---|---|---|
| AC-1 | Trait + client shape: LiveExecRouter exists, FakeTransport implements it, constructor-injected | `live_exec_adversarial::live_exec_router_trait_exists` | **PASS** |
| AC-2 | SecretString Debug/Display emit `"<redacted>"`, serde refused | `core::secret::secret_never_logged_or_serialized` | **PASS** |
| AC-3 | Missing fails closed — never default/empty key | `agent::secret::missing_secret_fails_closed_env`, `missing_secret_fails_closed_local_file`, `empty_env_var_is_missing` | **PASS** |
| AC-4 | AccountReader parses Decimal from recorded fixture, free+locked split | `live_exec_adversarial::account_reader_parses_decimal` | **PASS** |
| AC-5 (ADVERSARIAL) | Filter pre-validation: under-min / bad-step → ExecError::FilterReject, zero network requests | `filters::under_min_notional_fails_fast`, `filters::bad_lot_step_rejected` | **PASS** |
| AC-6 | HMAC-SHA256 signer against Binance docs public vector; key never in Debug | `sign::signer_reproduces_fixed_vector`, `clock::clock_skew_resyncs_then_halts` | **PASS** |
| AC-7 (ADVERSARIAL) | Valid order observably submitted exactly once to FakeTransport, newClientOrderId + OrderAck round-trip | `live_exec_adversarial::order_observably_submitted_once` | **PASS** |
| AC-8 (ADVERSARIAL) | Ambiguous timeout: status queried BEFORE retry; exhaustion → halt, never silent | `live_exec_adversarial::ambiguous_timeout_queries_before_resubmit` | **PASS** |
| AC-9 (ADVERSARIAL/STATIC) | No f64 in order/balance/cap/filter/tolerance/rounding — zero grep matches; `#![deny(clippy::float_arithmetic)]` compile-time gate | `live_exec_adversarial::decimal_only_compile_time_guard` + grep clean | **PASS** |
| AC-10 (ADVERSARIAL) | SOFT: N=2 consecutive reads → LedgerImbalance trip; single read → no trip; reset-on-clear; HARD: unknown position → immediate trip; paper=None → noop | `live_reconcile_adversarial::{reconcile_divergence_trips_halt, soft_once_then_clear_no_halt, soft_divergence_counter_resets_on_clear, reconcile_unknown_position_hard_trips, hard_immediate_trips_on_first_read, tolerance_boundary_exact_no_halt, paper_mode_reconcile_is_noop}` | **PASS** |
| AC-11 (ADVERSARIAL) | Cap rejects over-notional (5 cases incl. boundary==cap ALLOWED, >cap REJECTED); FakeTransport records zero requests | `cap::exec_side_cap_rejects_over_notional`, `live_exec_adversarial::cap_exceeded_fake_transport_receives_zero_requests` | **PASS** |
| AC-12 (ADVERSARIAL) | No real exchange / no real key in CI; keys unset; Network::default()==Testnet; zero mainnet calls | `live_exec_adversarial::no_real_exchange_no_real_key_in_ci`, `endpoint::default_endpoint_is_testnet`; all three testnet-live tests #[ignore]-gated | **PASS** |
| AC-13 | Testnet rehearsal recipe produced (M-DEV-F1 code wired, 3 #[ignore]-gated tests); operator run is F2 gate | `binance_testnet_live::{place_order_testnet, account_read_testnet, reconcile_no_divergence_testnet}` — 0 passed; 3 ignored (clean skip) | **PASS — code gate** |
| AC-14 | Parse-rejection untouched: no Mode::Live variant, config.rs:660-668 unchanged | `agent::config::tests::t12_mode_live_is_rejected` — green | **PASS** |
| AC-15 | Anchor-neutral: no anchors.toml row, no anchor SHA mutation | `bash scripts/verify_anchors.sh` → 119/119 PASS | **PASS** |

---

## 5. AQ-1 Contract Verification by Inspection + Test

**SOFT debounce N=2 with reset-on-clear:**
- `reconcile_divergence_trips_halt`: ledger 0.002 BTC vs exchange 0.001 BTC (delta 40 USDT >> $1 tolerance). First read: `!ks_clone.is_tripped()` asserted. Second read: `ks_clone.is_tripped()` asserted. Genuine kill-switch instance used.
- `soft_once_then_clear_no_halt`: single in-tolerance read does not trip through two calls.
- `soft_divergence_counter_resets_on_clear`: diverge → in-tolerance → diverge again; proves counter reset so second cycle needs N=2 fresh reads.

**HARD immediate bypass (no debounce):**
- `reconcile_unknown_position_hard_trips`: exchange reports DOGE (120 qty × ~$0.15 ≈ $18 >> dust floor). Ledger empty. Single `check_live_divergence` call → `ks_clone.is_tripped()` asserted.
- `hard_immediate_trips_on_first_read`: exchange reports 0.1 BTC (= $4000) unknown to ledger. Single call → tripped.

**Tolerance-boundary-exact no-halt:**
- `tolerance_boundary_exact_no_halt`: ledger 0.001025 BTC, exchange 0.001 BTC, mark $40,000. Delta = 0.000025 × 40,000 = $1.00 == tolerance. Two consecutive calls: `!ks_clone.is_tripped()` asserted after each. Condition is `delta > tolerance` (strict), confirmed.

**Paper-None byte-unchanged:**
- `paper_mode_reconcile_is_noop`: `AccountReader = None` path — wildly divergent ledger (999 BTC) does not trigger halt. No kill-switch reference reachable. Existing `t26_*` tests green (collateral verification via 132 agent-suite passes).

All four AQ-1 sub-contracts verified by genuine assertion against real kill-switch state.

---

## 6. Security Inspection (cite file:line for each)

**(a) No real key material in any fixture:**
- `crates/exec/src/live/sign.rs:62` — uses Binance API-docs public example secret `"NhqPtmdSJYdKjVHjA7PZj4Mge3R5YNiP1e3UZjInClVN65XAbvqqM6A7H5fATj0j"` (documented as a public example at the Binance docs URL cited inline at sign.rs:53). This string is labeled "a public example, never a live secret" in both code comment and test docstring.
- All other fixtures use `"FAKE_TESTNET_KEY_DO_NOT_USE"` / `"FAKE_TESTNET_SECRET_DO_NOT_USE"` (self-evidently non-credentials).
- Grep for any 32+ char alphanumeric patterns resembling real Binance key format found only the above doc-example location. **CLEAN.**

**(b) `SecretString` never in Debug/Display/Serialize/log:**
- `crates/core/src/secret.rs:70-93` — `Debug` and `Display` both emit `"<redacted>"` (hardcoded); `Serialize` impl returns `Err(serde::ser::Error::custom(...))` — no code path serializes the value.
- `crates/core/src/secret.rs:131-156` — `secret_never_logged_or_serialized` test: `format!("{:?}", s)` == `"<redacted>"`, `format!("{}", s)` == `"<redacted>"`, `serde_json::to_string(&s).is_err()`. Verified by running test: **PASS.**
- Plaintext reachable only via `expose_secret` (`secret.rs:56`) and `expose_str` (`secret.rs:64`). Both are `pub` but carry a caller contract in doc-comments.

**(c) No signature or full signed query string logged:**
- `crates/exec/src/live/mod.rs:189-196` — `signed_query` method has explicit doc comment: `"NEVER log the returned string — it contains the signature."` The signed string is passed as `body(signed)` to reqwest only.
- `crates/exec/src/live/mod.rs:308` — logging call: `debug!(endpoint = self.endpoint.label, symbol, "POST /api/v3/order")` — logs endpoint label + symbol only, NOT the signed query.
- Grep of all tracing calls in `crates/exec/src/live/`: none log `signed`, `signature`, `query`, `api_key`, or `api_secret`. **CLEAN.**

**(d) `Network::default()` is Testnet + test enforcing it:**
- `crates/exec/src/live/endpoint.rs:53-57` — `impl Default for Network { fn default() -> Self { Self::Testnet } }`.
- `crates/exec/src/live/endpoint.rs:69-78` — `default_endpoint_is_testnet` test: asserts `Network::default() == Network::Testnet`, `ep.label == "testnet"`, `ep.base_url == "https://testnet.binance.vision"`, and `!ep.base_url.contains("api.binance.com")`. Test: **PASS.**

**(e) Zero mainnet URLs reachable in CI:**
- `crates/exec/src/live/endpoint.rs:46` — `"https://api.binance.com"` appears ONLY in the `Network::Mainnet` enum arm. No CI test path constructs `Network::Mainnet`. The testnet-live suite tests (`binance_testnet_live.rs`) assert `ep.label == "testnet"` before any request and are `#[ignore]`-gated. No other reference to `api.binance.com` in exec, agent, or core. **CLEAN.**

**(f) `mode=live` parse-rejection untouched — `t12_mode_live_is_rejected` green:**
- `crates/agent/src/config.rs` — `t12_mode_live_is_rejected` (inline test): **PASS** (verified by `cargo test -p agent t12_mode_live_is_rejected`).
- No `Mode::Live` variant added anywhere in F1 scope. `config.rs:660-668` parse-rejection stays in force.

**(g) Fails-closed: no-secret constructor paths error (never panic/default):**
- `crates/exec/src/live/mod.rs:135-140` — `BinanceSpotExecClient::connect` calls `secrets.get("BINANCE_API_KEY")` and maps `Missing` to `ExecError::Auth(...)`, then returns `Err`. No default key, no empty key, no silent unauthenticated path.
- `crates/agent/src/secret.rs:37-43` — `EnvSecretSource::get`: `Err(SecretError::Missing)` on absent or empty env var.
- Tests `missing_secret_fails_closed_env`, `empty_env_var_is_missing`, `missing_secret_fails_closed_local_file`: all **PASS** with `Err(SecretError::Missing)` asserted.

**Overall security verdict: CLEAN on all 7 checks.**

---

## 7. Property / Fuzz Tests

_n/a_ — No proptest or cargo-fuzz suites for F1 (transport/client feature; adversarial matrix via unit tests with explicit parametrized cases is the chosen approach, per tasks.md).

---

## 8. Backtest Results

_n/a_ — F1 is a transport/client feature (no strategy, no sizing, no allocation decision). The live client is never on the backtest/report path (AC-15 / anchor-neutral by construction, ADR-0054 § Consequences). Baseline-equity-divergence gate: **N/A for F1 (justified)** — F1 has no sizing decision; the gate APPLIES to F2 per ADR-0054 § D4 and feature.md § non-negotiable 4.

---

## 9. Benchmarks

_n/a_ — No latency-sensitive paths touched by F1 (transport is a cold path; the signing function is a pure function not on a hot loop). No criterion suites added.

---

## 10. Anchor Verification

`bash scripts/verify_anchors.sh` → **ANCHORS PASS (119 / 119)**

AC-15 confirmed: F1 touches no `spec/anchors.toml` row and no anchor SHA. The live client is never on the hashed backtest-report path. Result: **anchor-neutral**.

Anchors column for `REQ-LIVE-EXEC-CLIENT-001`: `[]` (empty — correct by construction).

---

## 11. Spec-lint Gate

`python3 scripts/spec_lint.py` → **`spec-lint: FAIL (70 violations in 2 categories)`**

`python3 scripts/spec_lint.py --self-test` → **`spec-lint --self-test (status-drift): PASS`**

| Category | Current | Previous baseline (paper-mode-equity-wiring 2026-06-12) | Delta |
|---|---:|---:|---:|
| dead-link | 65 | 66 | -1 (improved) |
| trace-broken-path | 5 | 5 | 0 |
| **Total** | **70** | **71** | **-1** |

**Assessment:** Counts decreased (improved) vs most recent tester baseline. No new category introduced. No violations are in `live-exec-client-binance-spot/`. All violations are pre-existing — the dead-links are stale references in historical feature specs and ADRs (v25-kronos, crates/forecast/src/bin, /tmp/orch-diag screenshots, v1-5b-multi-venue, visual-fail-html-reporter); the trace-broken-paths are: `REQ-LAB-YAHOO-REALDATA-V0-1-4-001`, `REQ-VISUAL-FAIL-HTML-REPORTER-001` (×2), `REQ-QUEUE-STALENESS-RECONCILIATION-001`, `REQ-OPERATOR-LEDGER-SCHEMA-LINT-001`. **Does NOT block PASS.**

---

## 12. N/A Gates (Explicitly Recorded)

| Gate | Status | Justification |
|---|---|---|
| Baseline-equity-divergence e2e test | **N/A for F1** | F1 is transport only — no sizing decision, no allocation, no scale computation. The gate guards against "computed but never applied" (vol-overlay precedent). F1's analogue ("order actually left") is discharged by AC-7 + AC-11. Gate APPLIES to F2 per ADR-0054 § D4 + feature.md § non-negotiable 4. |
| Backtest scenarios | **N/A** | Live client is never on the backtest/report path (AC-15, anchor-neutral). |
| UI render-harness / cockpit-smoke | **N/A** | F1 has no UI surface. `cargo test -p ui --lib` → 447 passed; 0 failed (collateral proof). |
| Fuzz / proptest suites | **N/A** | No suites for this feature; adversarial matrix covered by parametrized unit tests. |
| Benchmarks | **N/A** | No hot paths touched. |
| Testnet rehearsal (M-DEV-F2) | **GATE TO F2 — operator-only** | The `binance_testnet_live` suite compiles and skips cleanly (0 passed; 3 ignored). The rehearsal pass requires operator-provisioned testnet keys; this is the F2-gate, not a CI gate. |

---

## 13. Developer Warning Residuals

Warnings in integration test files (not production code):
- `crates/exec/tests/live_exec_adversarial.rs:31` — `unused import: BinanceAccountResponse` (dead code in test helper, not production)
- `crates/exec/tests/live_exec_adversarial.rs:111,116` — `FakeAccountReader` struct/fn unused (defined but not used in this file's test scope)
- `crates/exec/tests/live_exec_adversarial.rs:131` — `FakeSecretSource` unused (imported but prepared for future test expansion)
- `crates/agent/tests/live_reconcile_adversarial.rs:27` — `HaltReason` unused import
- `crates/agent/tests/live_reconcile_adversarial.rs:56` — `FakeAccountReader::empty` unused

**Assessment:** All warnings are in `#[cfg(test)]` integration test files, not in production library code. `cargo clippy -p exec -p agent -p trading_core` (library code only) emits zero warnings. These test-file dead-code warnings are cosmetic and do not affect correctness or security. No action required before PASS.

---

## 14. Verdict

**`PASS`**

All 15 acceptance criteria are verified (8 adversarial confirmed by genuine kill-switch + transport-recording assertions). The full security inspection on all 7 checks is CLEAN — no secret material in any fixture, `SecretString` redaction proven by test, signed query never logged, `Network::default()` is Testnet enforced by a gate test, mainnet URL appears only in the typed enum, mode=live parse-rejection untouched, and all constructor no-secret paths return `Err` not panic/default. Suite counts: 904 passed, 0 failed, 6 ignored (all pre-existing or operator-gated). Anchors: 119/119 PASS. Spec-lint: 70 violations in 2 pre-existing categories, 1 improved vs prior baseline — no new entries, does not block. Zero new regressions vs main baseline.

---

## 15. Routing

`VERDICT → PASS`

F2 gate: the testnet rehearsal (M-DEV-F2 / AC-13) is an operator-only action (provision FAKE testnet keys, run `cargo test -p exec --test binance_testnet_live -- --ignored`). The tester does not run it. F2 does not start until the operator confirms AC-13 green.

---

## Appendix — Pre-existing Spec Debt

**dead-link (65 — all pre-existing, zero new):**
Historical: stale links in `spec/v0-paper-sma/reports/`, `spec/v05-composed-strategies/`, `spec/architecture/adr/0027-kronos-onnx-tract-integration.md`, `spec/chart-canvas-overhaul/feature.md` (`/tmp/orch-diag` screenshots), `spec/visual-fail-html-reporter/feature.md`, `spec/v1-5b-multi-venue/feature.md`, `spec/v2-llm-strategy/tasks.md`, `spec/v3-llm-forecaster/`, `spec/v3-volatility-forecaster/reports/`, `spec/ui-test-harness-viewport-matrix/tasks.md`.

**trace-broken-path (5 — all pre-existing, zero new):**
- `REQ-LAB-YAHOO-REALDATA-V0-1-4-001` arch path missing
- `REQ-VISUAL-FAIL-HTML-REPORTER-001` tests paths ×2 missing
- `REQ-QUEUE-STALENESS-RECONCILIATION-001` test path missing
- `REQ-OPERATOR-LEDGER-SCHEMA-LINT-001` test path missing

None are in `live-exec-client-binance-spot`. All carried from prior baseline without growing.
