---
slug: live-exec-client-binance-spot
mode: release
status: draft
audience: human-operator
updated: 2026-06-12
generated: 2026-06-12T16:00:00Z
trace: REQ-LIVE-EXEC-CLIENT-001
---

# F1 — Binance Spot live execution client + real-exchange reconciliation (TESTNET-FIRST) — release

## TL;DR

The repo can now *talk* to a real Binance exchange — authenticated order client, secret boundary, and a two-class kill-switch reconciler, all behind traits and testnet-default — but it still *cannot trade live*: `mode = "live"` is rejected at config parse, exactly as before, and arming is the next feature (F2).

## What changed

- **The execution substrate now exists.** F1 builds an authenticated `BinanceSpotExecClient` (signed REST: place MARKET / query status / cancel) behind a `LiveExecRouter` trait, a `SecretSource` boundary for API keys, an `AccountReader` for real balances, exchange-filter pre-validation, and a real-exchange reconciliation loop that trips the kill switch on divergence. None of this existed before — the repo had a read-only market-data feed and a self-referential equity heuristic, nothing that could place an order or read an account.
- **It cannot trade live, by design.** F1 adds **no** `Mode::Live` variant and does **not** un-reject `mode = "live"` — the parse-rejection (`config.rs:660-668`, test-pinned) stays in force. The 5-condition arming guard is F2; this feature only builds the cap *mechanism* the guard will later call. The only network the test suite can reach is testnet, and even that requires an out-of-band operator key + an opt-in toggle.
- **The safety nervous system is in place.** The reconciler now has two divergence classes: SOFT (a benign balance-timing mismatch, debounced over N=2 consecutive bars so a single in-flight fill does not false-halt) and HARD (an unknown exchange position — never benign — trips immediately). Both route to the existing supreme kill switch (`HaltReason::LedgerImbalance`). This is what every later live step (F2 arming, F3 canary) stands on.

## Why

The operator ratified the live-money boundary on 2026-06-12 (ADR-0054 § D7): the product *may eventually* run the passive buy-and-hold baseline live on Binance spot — operator-armed, capped, kill-switch-supreme — but none of the transport to do that existed. F1 builds exactly that transport and nothing more, **testnet-first**, so every later arming step has a real, proven client to gate against. The deliberate sequencing — substrate now, policy + arming in F2, canary in F3 — means a half-armed `Mode::Live` can never exist on `main`: the un-rejection and the arming guard land atomically in F2, never apart. (Source: `spec/live-exec-client-binance-spot/feature.md` § Why; `spec/architecture/adr/0054-mode-live-boundary.md` § D1/D2/D5.)

## Security — the section to trust first

This is a real-money substrate, so the security posture is the load-bearing claim, not the feature count. The tester's 7-point inspection is reproduced below with file:line citations; the presenter independently re-ran the greps and the redaction test against the on-disk tree at HEAD `414c18a`. **Every check is CLEAN.**

| # | Invariant | What was verified | Evidence (file:line) | Presenter re-check |
|---|-----------|-------------------|----------------------|--------------------|
| **a** | No real key in any fixture | All test secrets are self-evident placeholders (`FAKE_TESTNET_KEY_DO_NOT_USE`) or the Binance public docs example vector, labeled "a public example, never a live secret" in code + docstring. Grep for real-key-format strings found only the doc-example. | `crates/exec/src/live/sign.rs:62` (doc-example vector); `:53` (cited Binance docs URL) | grep clean — no live-key-format strings outside the labeled doc-example |
| **b** | `SecretString` unprintable + serde-refused | `Debug` + `Display` both emit `"<redacted>"` (hardcoded); `Serialize` returns an explicit error — no code path emits plaintext. Plaintext reachable only via `expose_secret`/`expose_str` (caller-contracted, consumed only by the signer). | `crates/core/src/secret.rs:73-84` (redaction); `:90-95` (serde refused); `:50-69` (expose contract) | confirmed verbatim by reading the file; `secret_never_logged_or_serialized` re-run → PASS |
| **c** | Signatures never logged | The signed query string carries the signature and is passed only as the request body. Every tracing call in `live/` logs the endpoint label + symbol only — never the signed string, signature, or keys. | `crates/exec/src/live/mod.rs:189-196` ("NEVER log the returned string"); `:308` (logs label + symbol only) | grep of all tracing calls in `live/` for `signed`/`signature`/`api_key`/`api_secret`/`query` → **zero matches** |
| **d** | Testnet default, type-enforced | `Network::default() == Testnet`; a gate test asserts the default label is `"testnet"`, base URL is `testnet.binance.vision`, and is not `api.binance.com`. "F1 ships testnet-only" is enforced, not hoped. | `crates/exec/src/live/endpoint.rs:53-57` (Default → Testnet); `:69-78` (`default_endpoint_is_testnet`) | `Self::Testnet` confirmed at `endpoint.rs:41,56`; test re-run → PASS |
| **e** | Mainnet URL only in the typed enum, zero CI paths | `api.binance.com` appears only in the `Network::Mainnet` enum arm; no CI test constructs `Network::Mainnet`. The testnet-live suite is `#[ignore]`-gated and asserts `"testnet"` before any request. | `crates/exec/src/live/endpoint.rs:46` (Mainnet arm only) | grep in `crates/exec/src/live/` → only the enum arm (`:46`), a doc comment (`:31`), a test assert (`:77`). **See honest note below.** |
| **f** | `mode=live` rejection untouched | F1 adds no `Mode::Live` variant; the parse-rejection stays in force and its guard test stays green. | `crates/agent/src/config.rs:660-668` (parse-rejection); `t12_mode_live_is_rejected` | re-run → **PASS** (1 passed) |
| **g** | Fails-closed constructors | `BinanceSpotExecClient::connect` maps a missing key to `Err(ExecError::Auth)` and returns `Err` — never a default/empty key, never a silent unauthenticated request. The env source errors on absent *or empty*. | `crates/exec/src/live/mod.rs:135-140` (connect fails closed); `crates/agent/src/secret.rs:37-43` (env source) | `no_real_exchange_no_real_key_in_ci`, `missing_secret_fails_closed_*` re-run → PASS |

**Honest note on (e).** There is a fourth `api.binance.com` in the tree, at `crates/agent/src/config.rs:34` — but it is the **pre-existing read-only `BinanceFeed` market-data REST URL** (the unauthenticated klines/exchange-info feed), not the F1 live exec client. It signs nothing and places nothing. The tester's "(e) CLEAN" finding is specifically about the live exec path, which is correct; this line is outside F1's scope and unchanged by it. Surfaced here so the operator sees the full grep, not a curated one.

**Security verdict: CLEAN on all 7 checks** — no secret material in any fixture, redaction proven by a re-run test, signed query never logged, testnet default enforced by a gate test, mainnet URL confined to the typed enum, parse-rejection untouched, and every no-secret constructor path returns `Err` rather than defaulting.

## What you can do now

| Action | Command |
|--------|---------|
| Run the testnet rehearsal — **THE GATE TO F2** (operator-only; full recipe below) | `cargo test -p exec --test binance_testnet_live -- --ignored --nocapture` (with testnet env set) |
| Prove the CI suite touches no real exchange / no real key | `cargo test -p exec --test live_exec_adversarial` |
| Prove the two-class reconciler trips correctly (SOFT debounce / HARD immediate) | `cargo test -p agent --test live_reconcile_adversarial` |
| Confirm `mode = "live"` is still rejected at parse | `cargo test -p agent t12_mode_live_is_rejected` |
| Confirm research + backtests are byte-unchanged | `bash scripts/verify_anchors.sh` (expect `ANCHORS PASS (119 / 119)`) |

## Live demo

F1 is a transport/client feature with no binary of its own and no backtest surface — there is nothing to `cargo run`. The ground-truth evidence is the adversarial CI suites + the security greps + the anchor gate, all run live by the presenter against the on-disk tree at HEAD `414c18a`. Full captured output: `artifacts/live-exec-client-binance-spot-2026-06-12/ground-truth-runs.txt`.

```
$ cargo test -p exec --test live_exec_adversarial -- --nocapture
running 7 tests
test no_real_exchange_no_real_key_in_ci ... ok
test decimal_only_compile_time_guard ... ok
test cap_exceeded_fake_transport_receives_zero_requests ... ok
test account_reader_parses_decimal ... ok
test order_observably_submitted_once ... ok
test live_exec_router_trait_exists ... ok
test ambiguous_timeout_queries_before_resubmit ... ok
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo test -p agent --test live_reconcile_adversarial -- --nocapture
running 7 tests
test tolerance_boundary_exact_no_halt ... ok
test soft_once_then_clear_no_halt ... ok
test paper_mode_reconcile_is_noop ... ok
test hard_immediate_trips_on_first_read ... ok
test reconcile_divergence_trips_halt ... ok
test reconcile_unknown_position_hard_trips ... ok
test soft_divergence_counter_resets_on_clear ... ok
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo test -p exec --test binance_testnet_live          # keys unset
running 3 tests
test account_read_testnet ... ignored, operator-only: requires BINANCE_EXEC_LIVE_TESTNET=1 + testnet keys
test place_order_testnet ... ignored, operator-only: requires BINANCE_EXEC_LIVE_TESTNET=1 + testnet keys
test reconcile_no_divergence_testnet ... ignored, operator-only: requires BINANCE_EXEC_LIVE_TESTNET=1 + testnet keys
test result: ok. 0 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out
```

Notice the last block: with keys unset (the CI default), the three live tests **skip cleanly** with the exact operator-gate message — no socket is opened. The first two blocks are the safety matrix (no real exchange/key in CI; the reconciler trips on real divergence and does not false-halt on a single transient read), all green.

## Screenshots

_n/a — F1 has no UI surface. The live cockpit is F3's monitoring concern. The tester recorded `cargo test -p ui --lib` → 447 passed / 0 failed (collateral proof the UI is untouched-green); there is no widget, layout, or theme change to capture._

## Verification matrix

All 15 acceptance criteria. The 8 adversarial ACs are named with their proving test. Evidence is verified live by the presenter where marked "(presenter ran live)"; the rest cite the tester report § 4 (`reports/test-2026-06-12-live-exec-client-binance-spot.md`).

| AC | Description | Status | Evidence |
|----|-------------|--------|----------|
| AC-1 | Trait + client shape: `LiveExecRouter` place/status/cancel; client + fake both impl it; base-URL+keys injected, no hard-coded venue | VERIFIED | `live_exec_router_trait_exists` PASS (presenter ran live, 7/7) |
| AC-2 (ADVERSARIAL) | `SecretString` `Debug`/`Display` emit `"<redacted>"`; serde refused | VERIFIED | `core::secret::secret_never_logged_or_serialized` PASS; `secret.rs:73-95` read + re-run by presenter |
| AC-3 | `Missing` fails closed — never a default/empty key | VERIFIED | `missing_secret_fails_closed_env`/`_local_file`, `empty_env_var_is_missing` PASS (tester § 4) |
| AC-4 | `AccountReader` parses `Decimal` from recorded fixture; free+locked split | VERIFIED | `account_reader_parses_decimal` PASS (presenter ran live) |
| AC-5 (ADVERSARIAL) | Under-min / bad-step order → `ExecError::FilterReject`, zero outbound requests | VERIFIED | `under_min_notional_fails_fast`, `bad_lot_step_rejected` PASS (tester § 4) |
| AC-6 | HMAC-SHA256 signer reproduces pinned vector; key never in `Debug` | VERIFIED | `signer_reproduces_fixed_vector`, `clock_skew_resyncs_then_halts` PASS (tester § 4) |
| AC-7 (ADVERSARIAL) | Valid order observably submitted exactly once with `newClientOrderId`; `OrderAck` round-trips ("did it leave" analogue) | VERIFIED | `order_observably_submitted_once` PASS (presenter ran live) |
| AC-8 (ADVERSARIAL) | Ambiguous timeout → status queried BEFORE retry; never blind-resubmit; exhaustion → halt, never silent | VERIFIED | `ambiguous_timeout_queries_before_resubmit` PASS (presenter ran live) |
| AC-9 (ADVERSARIAL/STATIC) | No `f64` in any order/balance/cap/filter/tolerance/rounding path | VERIFIED | `decimal_only_compile_time_guard` PASS (presenter ran live) + `#![deny(clippy::float_arithmetic)]` + grep clean |
| AC-10 (ADVERSARIAL) | SOFT N=2 → trip; single read → no trip; reset-on-clear; HARD unknown position → immediate trip; paper `None` → no-op | VERIFIED | 7-test `live_reconcile_adversarial` suite PASS (presenter ran live, 7/7) |
| AC-11 (ADVERSARIAL) | Cap rejects over-notional (5 cases incl. `== cap` allowed, `> cap` rejected); fake transport records zero requests | VERIFIED | `exec_side_cap_rejects_over_notional`, `cap_exceeded_fake_transport_receives_zero_requests` PASS (presenter ran live, the latter) |
| AC-12 (ADVERSARIAL) | No real exchange / no real key in CI; keys unset; `Network::default()==Testnet`; zero mainnet calls | VERIFIED | `no_real_exchange_no_real_key_in_ci`, `default_endpoint_is_testnet` PASS (presenter ran live); 3 testnet tests `#[ignore]`-gated |
| AC-13 | Testnet rehearsal recipe produced; 3 `#[ignore]`-gated tests; operator run is the F2 gate | VERIFIED (code gate) | `binance_testnet_live` → 0 passed / 3 ignored, clean skip (presenter ran live). Operator rehearsal = M-DEV-F2 below |
| AC-14 | Parse-rejection untouched: no `Mode::Live`, `config.rs:660-668` unchanged | VERIFIED | `t12_mode_live_is_rejected` PASS (presenter ran live) |
| AC-15 | Anchor-neutral: no `anchors.toml` row, no anchor SHA mutation | VERIFIED | `verify_anchors.sh` → `ANCHORS PASS (119 / 119)` (presenter ran live); trace `anchors = []` |

## Numbers that matter

- **Anchors: 119 / 119 PASS** — presenter ran `bash scripts/verify_anchors.sh` live at HEAD `414c18a`. F1 is anchor-neutral by construction (the live client is never on the backtest/report path).
- **Tests:**
  - Suite totals at verdict (tester § 3a, commit `dc3ef58`): **904 passed, 0 failed, 6 ignored** across `trading_core` (100), `exec` (38 + 3 ignored), `agent` (132 + 2), `audit` (187 + 1), `ui --lib` (447). The 6 ignored = 3 operator-gated testnet + 3 pre-existing doc-test stubs.
  - Re-verified post-hygiene (HEAD `414c18a`): `exec` adversarial **7/7**, `agent` reconciler **7/7**, `t12_mode_live_is_rejected` **1/1**, testnet suite **0 passed / 3 ignored** — all run live by the presenter.
  - F1 added **48 new tests** (38 exec + 7 reconciler + 3 ignored testnet).
- **Dep gate:** `hmac` + `hex` (the only two new crates, RustCrypto, MIT/Apache-2.0) introduce **zero** new advisories / bans / license issues. The one advisory (`paste` unmaintained) and one license gap (`polars-arrow-format`) are **pre-existing**, present before F1 (tester § 2).
- **Divergence gate (the baseline-equity-divergence non-negotiable): N/A for F1 — justified, APPLIES to F2.** F1 ships no sizing decision — it is transport: it places the order it is handed, reads balances, reconciles. There is no allocation, weight, rebalance, or `scale` that could be "computed but never applied" (the exact vol-overlay failure mode the gate guards). The gate has no decision variable to bind here. F1's analogue of "the order actually left the process" is discharged by AC-7 (observably submitted) + AC-11 (over-cap/under-filter observably does NOT reach the wire) + the AC-13 rehearsal. The gate **APPLIES to F2** (ADR-0054 § D4), which carries the inception-allocation + monthly-rebalance sizing decision, with the e2e proof `passive_inception_diverges_from_flat_baseline`. This is the honest read — you cannot prove "the allocation moved capital" in a feature that has no allocation. (Source: feature.md § non-negotiable 4; tester § 12.)
- **Spec-lint:** `spec-lint: FAIL (70 violations in 2 categories)` — 65 dead-link + 5 trace-broken-path, all pre-existing, **down 1** from the `audit-2026-06-12` baseline of 71. Zero violations in `live-exec-client-binance-spot/`. Does not block (presenter re-ran live).
- **Perf / benchmarks:** _n/a — no hot path touched (the signer is a pure cold-path function; transport is a cold path). No criterion suites added (tester § 9)._

## Open decisions

1. **Ship F1 — the live execution substrate?** Everything verifies green: all 15 ACs (8 adversarial confirmed by genuine kill-switch + transport-recording assertions), 119/119 anchors, the full 7-point security inspection CLEAN, suite 904/0 at verdict and re-verified post-hygiene, dep-gate clean (hmac/hex add nothing), spec-lint down 1. F1 cannot trade live — `mode = "live"` stays rejected at parse — so approval ships a *capability*, not a live position. **A "yes" carries one follow-up cost you control:** the testnet rehearsal (M-DEV-F2, the recipe below) **must be green before F2 dispatches**. It can run before or after you tick — but F2 (the arming guard + passive policy) does not start until you confirm that single green run on fake money.

## Operator gate — testnet rehearsal recipe (M-DEV-F2 / AC-13) — THE GATE TO F2

This is the deck's centerpiece and the one thing only you can do. It runs the full place → status → cancel → account-read → reconcile pipeline against `testnet.binance.vision` with **fake money**. Three facts to be crystal clear about, up front:

- **Fake money only.** The keys are Binance *testnet* keys (from `testnet.binance.vision`, free, fake balances). No real funds, no mainnet, ever — the suite asserts the endpoint label is `"testnet"` before the first request and refuses otherwise.
- **The repo never sees the keys.** They live in your shell environment for the duration of the run and nowhere else — never a file in the repo, never an argument, never logged (`SecretString` redacts; the signed query is never logged).
- **This single green run is what unlocks F2.** F2 (the arming guard + the passive baseline policy) does not dispatch until you confirm this.

**Command**

```bash
# OPERATOR-ONLY — fake testnet money. The assistant never runs this.
export BINANCE_TESTNET_API_KEY=<your testnet key>       # from testnet.binance.vision
export BINANCE_TESTNET_API_SECRET=<your testnet secret>
export BINANCE_EXEC_LIVE_TESTNET=1
cargo test -p exec --test binance_testnet_live -- --ignored --nocapture
```

**Steps**

1. Go to `https://testnet.binance.vision`, sign in (GitHub auth), and **Generate HMAC_SHA256 Key**. This gives a testnet API key + secret backed by fake money — it is free and disposable. (If your fake balance is empty, use the testnet faucet on the same site to mint fake USDT/BTC.)
2. Export the three env vars above in the shell you will run the test from. The `BINANCE_EXEC_LIVE_TESTNET=1` toggle is the opt-in — without it the suite is a no-op even with `--ignored`.
3. Run the command. It exercises one **MARKET BUY of 0.001 BTCUSDT**, then queries its status, then an account-balance read, then the reconcile check (asserts no `LedgerImbalance`). The order auto-cancels / is left flat — no standing position remains.
4. Read the output: all three tests (`place_order_testnet`, `account_read_testnet`, `reconcile_no_divergence_testnet`) should report `... ok`.

**Timing**

- ~10–30 seconds end-to-end (a handful of REST round-trips to testnet). Not a long-running job — no `watch` block needed.

**Expected result**

- `test result: ok. 3 passed; 0 failed; 0 ignored` — the full pipeline green on `testnet.binance.vision`.
- The MARKET BUY 0.001 BTCUSDT acks with a `newClientOrderId`; status query returns FILLED (or NEW→FILLED); the account read returns your fake balances parsed as `Decimal`; the reconcile compare matches (no `LedgerImbalance` trip).
- No mainnet host is ever dialed (the suite asserts `"testnet"` before any request).

**Failure diagnosis**

| Symptom | Likely cause | What it means |
|---------|-------------|---------------|
| `0 passed; 3 ignored` (skip line printed) | `BINANCE_EXEC_LIVE_TESTNET=1` not set, or keys absent/empty | The opt-in toggle or keys are missing — **not a failure**, the suite is a no-op. Set all three env vars. |
| `ExecError::Auth` / HTTP -2014 / -2015 | bad or wrong-network key | Key is mistyped, revoked, or a mainnet key. Re-generate an **HMAC_SHA256** key on `testnet.binance.vision`. |
| Test panics on `endpoint.label == "testnet"` | safety guard tripped | The suite refused to run against a non-testnet endpoint. This is the safety net working; it should never fire with the stock suite. |
| `ExecError::Transport` / timeout | network / testnet reachability | `testnet.binance.vision` unreachable or down. Retry; check connectivity. |
| `ExecError::FilterReject` | testnet lot/notional filter on 0.001 BTCUSDT | Testnet filters occasionally differ from mainnet. Re-run; if persistent, the suite's order size needs a bump (route back). |
| `ExecError::ClockSkew` / -1021 | host clock drift > recvWindow | Your machine clock is off. Sync system time (NTP) and re-run — the client also auto-resyncs once. |

**Cleanup**

- `unset BINANCE_TESTNET_API_KEY BINANCE_TESTNET_API_SECRET BINANCE_EXEC_LIVE_TESTNET` when done (clears the keys from your shell).
- No repo state is written. The testnet order is on fake money and leaves no standing position; nothing to revert in the tree.
- Optionally revoke the testnet key on `testnet.binance.vision` if you do not plan to reuse it for the F2 canary.

## Approval

Approving this deck **ratifies F1's gated evidence** — the substrate is built, secured, and proven against the faked-I/O matrix + the anchor/lint/dep gates. It does **not** ship a live position (`mode = "live"` stays rejected). The testnet rehearsal (M-DEV-F2, above) can run before or after you tick this, but **must be green before F2 dispatches** — it is the hard gate to the arming guard.

- [x] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes / feedback

2026-06-12 — operator: **Approved — ship** (in-chat). Approval ships the capability; `mode = "live"` stays parse-rejected. The M-DEV-F2 testnet rehearsal remains the GATE TO F2 dispatch.

## Feedback log

_no feedback yet_

## Changelog

- 2026-06-12 (presenter): initial release deck for F1 (first build feature of the live-money program). Security section top-billed (tester a–g inspection with file:line, presenter re-ran the redaction test + the mainnet/signature greps live). Verification matrix covers all 15 ACs (8 adversarial named); evidence verified live by the presenter at HEAD `414c18a` (anchors 119/119, exec adversarial 7/7, reconciler two-class 7/7, `t12_mode_live_is_rejected` PASS, testnet suite clean-skip 0/3). Testnet rehearsal recipe (M-DEV-F2, the F2 gate) authored with all six sections + the failure table. Divergence-gate N/A-for-F1 justified (APPLIES to F2). Honest note added on the pre-existing `config.rs:34` market-data mainnet URL (outside F1 scope). All three approval boxes ship UN-ticked. Raw runs: `artifacts/live-exec-client-binance-spot-2026-06-12/ground-truth-runs.txt`.
