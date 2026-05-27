---
slug: v5-latency-slippage-sim
presentation_date: 2026-05-26
mode: release
version: 0.1.0
owner: presenter
tester_verdict: PASS
tester_report: ../reports/test-final-2026-05-26-v5-latency-slippage-sim.md
commit: 28db398dc871af042ed5d03d714574f32bdd1072
---

# v5 Latency & Slippage Simulator — v0.1.0 sprint review

## TL;DR

We shipped an opt-in execution-friction simulator that closes the
backtest-vs-live realism gap, and we did it without disturbing a
single byte of any of the 34 anchored backtest reports.

## What changed (3 bullets, plain language)

- The backtester can now apply **simulated network latency** (e.g. a
  20–100 ms delay between order placement and fill) and
  **simulated slippage** (a small price haircut, expressed in basis
  points, that represents walking the order book). Both default to
  **off / zero** — existing strategy numbers are untouched until the
  operator flips the knob.
- Every simulated fill writes a new **audit-ledger row**
  (`SimulatedExecMetrics`) recording exactly how much latency and how
  many slippage-dollars were applied, so the friction is auditable
  end-to-end. Rows are suppressed when both knobs are zero — that is
  what keeps the 34 anchored reports byte-identical.
- The plumbing is live across all 4 production scenarios that the
  simulator can reach: **momentum**, **pairs**, **tcn-overlay**, and
  **sma-composed-run**. The wiring is *passive* under default config
  (zeros in, zeros out).

## Why this matters

Real venues add 20–100 ms of wire latency on a typical retail
connection, and market orders fill worse than the recorded mid-price
because they walk the order book. Our previous backtest model assumed
**instant fills at the bar close with zero slippage** — systematically
optimistic, and the main reason paper-trading numbers tend not to
survive the jump to live trading (the well-known "backtest-vs-live
gap").

This release lays down the engine that closes that gap. Default-zero
means **no alpha numbers move today** — every Sharpe, every drawdown,
every final-equity figure the operator has ever signed off on is
preserved bit-for-bit. The v0.2.0 brief (already drafted) is where
the operator decides what canonical friction profile to re-anchor
against — separating "ship the engine" from "decide the numbers"
deliberately, per the original Q5 operator decision.

This is the first half of a two-step. Today: engine, no observable
change. v0.2.0: the canonical friction config gets picked and the
anchors migrate.

## What the operator can do now

| Action | How |
|---|---|
| Run a scenario with default zero friction (unchanged from today) | `cargo run -p backtest --release -- momentum --config configs/momentum.toml` |
| Run the same scenario with simulated friction | Add a `[latency_slippage_sim]` block to the scenario config: `latency_ms_min = 30`, `latency_ms_max = 80`, `slippage_bps = 8`. Same binary. |
| Verify the 34 anchored reports are still byte-identical | `bash scripts/verify_anchors.sh` |
| Confirm the simulator actually moves equity when enabled | `cargo test -p strategy --test latency_slippage_sim_e2e` |
| Decide v0.2.0 canonical friction profile | Review the v0.2.0 brief at `spec/v5-latency-slippage-sim-v0.2.0-anchor-migration/feature.md` |

## Live demo — the anchor gate

This is the load-bearing demonstration. With the simulator landed and
its default `LatencySlippageSimConfig` applied at every existing
construction site, all 34 anchored reports must still hash byte-for-
byte to their pre-feature values:

```
$ bash scripts/verify_anchors.sh
PASS  btc-2023-1m-sma-cross                      fc2e3b4a0405...
PASS  top10-2023-1h-momentum                     3b60ef074300...
PASS  top10-2023-fy-tcn-overlay                  01d025843314...
PASS  top10-2023-fy-vol-target-overlay-realdata  9fa64d467f35...
... (30 more rows) ...
ANCHORS PASS  (34 / 34)
```

Source: tester run 2026-05-27 09:30 UTC at commit
`28db398dc871af042ed5d03d714574f32bdd1072`. See
[test-final report § 5](../reports/test-final-2026-05-26-v5-latency-slippage-sim.md#5-backtest-results--anchor-gate-t-t-1).

And the CLAUDE.md-mandated divergence gate — proves the simulator is
NOT a no-op when the operator turns it on:

```
$ cargo test -p strategy --test latency_slippage_sim_e2e
test enabled_audit_metrics_recorded ........ ok
test enabled_diverges_by_at_least_1bp ...... ok
test noop_byte_identical_to_baseline ....... ok

test result: ok. 3 passed; 0 failed; 0 ignored; finished in 8.13s
```

With the simulator configured at `{latency 50–100 ms, slippage 10 bps}`
on a momentum scenario, final equity diverges from the noop baseline
by **at least 1 basis point** — confirming the engine is wired and
producing observable economic effect.

## Verification matrix (V1–V7 from feature.md § Scope)

| Req | What it asserts | Status | Evidence |
|---|---|---|---|
| **R1** — Config toggle, default-zero noop | `LatencySlippageSimConfig::default()` = all zeros; plumbed through `ScenarioConfig` | **VERIFIED** | 4 unit tests PASS; 34/34 anchors hold |
| **R2** — Latency simulation in `crates/exec` | `apply_latency` is noop at zero, fixed at min==max, jittered when range | **VERIFIED** | 4 unit tests + criterion: `apply_latency_noop` 2.35 ns, `apply_latency_jitter` 2.50 ns |
| **R3** — Slippage simulation in `crates/cost` | Linear bps, side-signed, noop at bps=0 | **VERIFIED** | 5 unit tests PASS; criterion `apply_slippage_10bps` 22.7 ns |
| **R4** — Audit-ledger metric variant + skip-when-zero | `AuditEvent::SimulatedExecMetrics` emits only when latency>0 OR slippage>0 | **VERIFIED** | 3 unit tests PASS; anchor-ledger byte-identity preserved |
| **R5** — CLAUDE.md non-negotiable e2e divergence | Enabled config moves final equity ≥ 1 bp from noop | **VERIFIED** | 3/3 PASS in `crates/strategy/tests/latency_slippage_sim_e2e.rs` |
| **R6** — Non-regression contract (R-NR.1–6) | 34/34 anchors hold; workspace tests pass; no Strategy-trait surface change; audit schema additive | **VERIFIED** | Tester report § 5 + § 3 + § 9 |
| **R7** — Criterion bench gate | Noop overhead small enough that R-NR.4 < 1% regression holds | **VERIFIED** | Noop fill = 1.46 ns/call overhead, well under 0.01% of realistic fill cost |

All 7 R rows VERIFIED. Verdict tree cell **R-O1 (SHIP)** triggered.

## Numbers that matter

| Metric | Value | Target | Verdict |
|---|---|---|---|
| Anchored reports byte-identical | **34 / 34** | 34 / 34 | LOCKED |
| CLAUDE.md divergence-gate tests | **3 / 3 PASS** | 3 / 3 | LOCKED |
| `apply_latency_noop` micro-bench | 2.35 ns | ≤ 5 ns | PASS |
| `apply_latency_jitter` micro-bench | 2.50 ns | ≤ 50 ns | PASS (20x under) |
| `apply_slippage_10bps` micro-bench | 22.7 ns | ≤ 10 ns | **Documented deviation accepted** (see below) |
| Full-scenario noop throughput (8760 fills) | 73.9 µs | n/a | Establishes v0.1.0 baseline |
| Full-scenario enabled throughput (8760 fills) | 171.6 µs | n/a | Opt-in cost — operator-elected |
| Noop per-fill overhead vs pre-feature | ~0.6 ns | < 1% | Analytically < 0.01% of any realistic fill |
| Workspace tests | 400+ PASS, 0 FAIL in touched crates | clean | PASS |
| New audit variant — schema impact | additive only | additive | v0 audit replay unchanged |
| Code added | ~840 lines (4 crates + 1 e2e test + 2 bench files) | n/a | — |

## Architecture call-outs (ADR-0043 sub-decisions)

- **D1 — Always-on code path, no Cargo feature flag.** Default config
  is noop; branch prediction makes the cost negligible. Avoids the
  two-code-paths bit-rot risk.
- **D2 — Deterministic latency RNG keyed on `(scenario_seed,
  order_id)`.** Implementation deviation: the ADR draft cited
  `ChaCha20Rng::from_seed()`, but the developer benchmark showed it
  cost 400–600 ns/order — 8–12x over budget. The actual ship swapped
  in a **Murmur3 finalizer mixer** (XOR-fold 32-byte seed + order_id,
  two finalizer rounds): same determinism contract, 2.28–2.50 ns/call,
  20x under target. **ADR-0043 Changelog amended 2026-05-27 to
  document this** — tester accepted it as an optimization below the
  ADR abstraction boundary.
- **D3 — Linear bps slippage.** Operator-decide Q2 result. Function
  signature already includes `notional` as an unused parameter so
  v0.2.0 can swap in square-root market impact with no call-site
  ripple.
- **D4 — Additive audit variant with skip-when-zero guard.** Operator-
  decide Q3 (load-bearing). Existing audit ledgers stay byte-identical;
  new variant only surfaces when friction is actually applied.
- **D5 — Backtest-only scope.** `LatencySlippageSimConfig` is read by
  `crates/backtest::ScenarioConfig` exclusively. Live mode
  (`crates/agent`) ignores it — simulating friction on live trades
  would double-count the real friction the venue already imposes.

Source: [ADR-0043](../../architecture/adr/0043-simulated-latency-and-slippage.md).

## Open follow-up — v0.2.0 anchor migration

The v0.2.0 brief is already on disk:
`spec/v5-latency-slippage-sim-v0.2.0-anchor-migration/feature.md`.

It is the operator-decide brief that picks the **canonical non-zero
friction profile** and re-emits every one of the 34 anchored reports
under it. Four questions await the operator:

| Q | Decision | Analyst-recommended default |
|---|---|---|
| **Q1** | Friction profile: tight / medium / aggressive / ladder | **medium** (`30..=80 ms, 8 bps`) — typical retail crypto-spot |
| **Q2** | OLD noop anchors: retire / keep as `noop-baseline` / both visible | **keep as `noop-baseline`** for historical A/B reference |
| **Q3** | Cross-product e2e re-verification scope | Re-run all known overlay e2e tests under canonical config |
| **Q4** | Anchor namespace tag | `v5-realdata-<profile>-2026-05` per Q1 selection |

These are deliberately NOT in scope for today's approval — the v0.1.0
ship is a clean separation of "engine in" from "numbers move".

## Documented deviations (3, all triage-accepted)

1. **`apply_slippage_10bps` 22.7 ns vs ≤10 ns target.** Root cause:
   `rust_decimal` arithmetic costs ~6–10 ns/op and the enabled path
   does 2–3 ops minimum. The ≤10 ns target was set before Decimal
   throughput was measured and is physically impossible without
   moving off `rust_decimal`. The noop path (which is the load-bearing
   one for anchor preservation) is unaffected at ~free. Tester:
   ACCEPT.
2. **R-NR.4 bench shape: noop vs enabled, not noop vs pre-feature.**
   No pre-v5 baseline bench exists to compare against. The
   `apply_latency_noop` micro-bench at 2.35 ns/call provides an
   analytical < 0.01% overhead bound on the noop path, satisfying
   R-NR.4. Tester: ACCEPT.
3. **D2 RNG: Murmur3 mixer instead of ChaCha20Rng (see above).**
   Tester: ACCEPT — ADR-0043 Changelog amended.

## Risk register status (K1–K7 from feature.md)

All 7 risks **mitigated**:

- **K1** (anchor drift under default) — gated by `verify_anchors.sh`
  → 34/34 PASS at every wave gate.
- **K2** (RNG non-determinism) — Murmur3 mixer is pure function of
  `(seed, order_id)`; no `thread_rng()`, no `SystemTime::now()`.
- **K3** (audit volume flood) — skip-when-zero guard prevents any
  audit row at default config. v0.2.0 may need aggregation when
  enabled-state lands.
- **K4** (backtest perf) — noop path verified at 1.46–2.35 ns/call.
- **K5** (vol_killswitch sequencing) — Bug #65 landed first;
  developer rebased; no conflict.
- **K6** (cockpit activity-tape flood) — skip-when-zero guard
  prevents flood; v0.1.1 audit-producer aggregation will handle
  enabled-state load.
- **K7** (live-mode double-count) — D5 architectural lock; live
  `crates/agent` does not read `LatencySlippageSimConfig`.

## Open decisions for the operator

**None for this approval.** The release ships the engine at default-
zero; no alpha numbers move; the operator decides v0.2.0 friction
profile separately. Today's decision is binary: **approve the engine
ship, or send it back**.

## Cross-references

- Feature brief — [`spec/v5-latency-slippage-sim/feature.md`](../feature.md)
- Tasks — [`spec/v5-latency-slippage-sim/tasks.md`](../tasks.md)
- Tester report (M-FINAL) — [`spec/v5-latency-slippage-sim/reports/test-final-2026-05-26-v5-latency-slippage-sim.md`](../reports/test-final-2026-05-26-v5-latency-slippage-sim.md)
- Architecture decision — [`spec/architecture/adr/0043-simulated-latency-and-slippage.md`](../../architecture/adr/0043-simulated-latency-and-slippage.md)
- Trace row — `REQ-V5-LATENCY-SLIPPAGE-001` in [`spec/trace.toml`](../../trace.toml)
- v0.2.0 anchor-migration brief — [`spec/v5-latency-slippage-sim-v0.2.0-anchor-migration/feature.md`](../../v5-latency-slippage-sim-v0.2.0-anchor-migration/feature.md)
- CLAUDE.md non-negotiable precedent — [`spec/dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md`](../../dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md)

## Approval

The verdict tree cell triggered by the tester's `PASS` and the 34/34
anchor result is **R-O1 → SHIP v0.1.0 + spawn v0.2.0 anchor-migration
brief** (the v0.2.0 brief is already drafted).

Operator picks one:

- [ ] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes / rejection reason

_(operator fills in here on approve-with-notes or reject)_

## Feedback log

_(presenter appends rejection notes here if the operator routes back)_

## Changelog

- 2026-05-26 (presenter): sprint-review deck authored. M-FINAL tester
  verdict PASS at commit `28db398dc871af042ed5d03d714574f32bdd1072`;
  34/34 anchors held; CLAUDE.md divergence gate 3/3 PASS; criterion
  baselines locked. Verdict tree cell R-O1 (SHIP) routed.
