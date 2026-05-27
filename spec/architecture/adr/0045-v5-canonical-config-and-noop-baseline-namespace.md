---
adr: 0045
title: v5 v0.2.0 canonical LatencySlippageSimConfig + noop-baseline namespace strategy
status: accepted
date: 2026-05-27
deciders: analyst M0 → operator M-OD → architect M-T1
supersedes: []
superseded_by: []
related:
  - "ADR-0043 simulated-latency-and-slippage (D1-D5 + Murmur3 amendment)"
  - "ADR-0038 spec-anchor-bounded-set-discipline (§ D6 anchor-additive contract)"
  - "ADR-0032 backtest-realdata-path-and-revision-pin"
---

# ADR-0045 — v5 v0.2.0 canonical config + noop-baseline namespace strategy

> ADR-0043 locked the simulator engine at **default-zero noop**. This
> ADR locks the **canonical non-zero values** every anchored backtest
> re-emits under, plus the two-namespace co-existence rule for the
> 34 OLD (noop) and 34 NEW (canonical) anchor rows.

## Context

v5-latency-slippage-sim v0.1.0 shipped 2026-05-26 (commit `a5f8647`)
with three knobs defaulted to zero. All 34 SHA-256 anchors in
[`spec/anchors.toml`](../../anchors.toml) stayed byte-identical — the
non-negotiable Wave A gate. But default-zero is operationally
meaningless: we shipped the engine without using it.

v0.1.0's R-O1 SHIP path explicitly spawned this v0.2.0 brief per Q5 =
(a) defer. The migration of 34 anchors to non-zero values is a
load-bearing operator decision deserving its own brief.

The v0.2.0 feature brief
([`spec/v5-latency-slippage-sim-v0.2.0-anchor-migration/feature.md`](../../v5-latency-slippage-sim-v0.2.0-anchor-migration/feature.md))
posed four operator-decide questions Q1-Q4. The operator resolved all
four on 2026-05-27 at analyst-recommended defaults via standing
Autoapprove. This ADR codifies those resolutions.

## Decision

### D1 — Canonical config = medium (Q1 = (b))

The new canonical `LatencySlippageSimConfig` literal is:

```rust
LatencySlippageSimConfig {
    latency_ms_min: 30,
    latency_ms_max: 80,
    slippage_bps:   8,
}
```

This means: latency sampled uniformly from `30..=80 ms` per fill via
the ADR-0043 D2 seeded sub-stream (Murmur3-mixer per the ADR-0043
2026-05-27 amendment), and a `+8 bps / -8 bps` linear price multiplier
per buy/sell side via ADR-0043 D3.

**Profile semantics**: matches realistic crypto-spot retail venue
conditions ca. 2024-2025 on top-3 venues (Binance / Coinbase / Kraken
non-API-tier-3). Not colocated, not stress-test — a sober production
default.

**Why medium over tight (a) / aggressive (c) / ladder (d)**:
- Tight (`20..=50 / 3 bps`) understates real retail friction.
  Colocated values, not the operator's actual production environment.
- Aggressive (`50..=150 / 15 bps`) is a stress-test config, not a
  default. Reserved for explicit stress runs in a future namespace
  (e.g. `v5_realdata_aggressive_2026_05`) when needed.
- Ladder (ship 3 namespaces at once) triples migration cost and
  dilutes "canonical" semantics. Defer to v0.3.0+ if A/B comparison
  needed.

### D2 — Two-namespace anchor co-existence (Q2 = (a))

`spec/anchors.toml` carries **both** the OLD 34 (noop) and NEW 34
(canonical) anchor rows, segregated by namespace. The OLD rows move
under the `noop-baseline` namespace; the NEW rows land under the
canonical namespace.

**Namespace pin chosen**: `v5-realdata-medium-2026-05` for the
canonical-friction rows. The `version` field on each NEW `[[anchors]]`
row uses the pin format **`<existing-version>+v5-realdata-medium-2026-05`**
(see D4 below for examples).

**Why both co-exist**:
1. The noop set is the **friction-free oracle** for divergence
   regression gates forever. Without it we cannot verify the simulator
   is still firing (R-NR.3 + the CLAUDE.md ≥ 1 bp non-negotiable both
   need a noop reference point).
2. Historical-evidence preservation: pre-v0.2.0 reports remain
   independently verifiable.
3. Storage cost is one file with 68 rows instead of 34. Clarity cost
   is zero given the explicit namespace separation.

The Sharpe-delta table (R5, dev-note authored by tester at M-FINAL)
becomes a **permanent regression artifact** — every future
strategy-anchor change re-renders the noop-vs-canonical delta and
checks it for K1 surprise (alpha inversion).

### D3 — Inverted-alpha scenarios flagged per scenario (Q3 = (b))

Tester at M-FINAL surfaces every scenario where `sharpe(canonical) <
0 ∧ sharpe(noop) > 0` (alpha flipped from positive to negative under
realistic friction) as a **K1-surprise candidate**. The operator
reviews each one and chooses retire / accept / refine in a follow-on
mini-brief.

**Why per-scenario, not auto-retire / blanket-ship**:
- Auto-retire is irreversible without re-spec work. A scenario that
  flipped at 8 bps may regain alpha at 3 bps — that's information.
- Blanket-ship sweeps the K1 problem under the rug. The whole point of
  v0.2.0 is honest reporting.
- Per-scenario forces an explicit operator acceptance trail —
  audit-friendly.

### D4 — Cross-feature e2e re-check is mandatory at Wave D (Q4 = (a))

Every overlay / sizing-modifier with an existing baseline-divergence
e2e test (per the CLAUDE.md non-negotiable, pattern reference
`crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`) re-runs
under the canonical config in Wave D. The "≥ 1 bp divergence vs noop"
assertion stays valid; absolute equity values shift per scenario.

**Inventory at 2026-05-27** (architect M-T1 survey of
`crates/strategy/tests/*_end_to_end.rs` + `latency_slippage_sim_e2e.rs`):

1. `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs` — v3 vol overlay
2. `crates/strategy/tests/vol_killswitch_overlay_end_to_end.rs` — Bug #65 fix
3. `crates/strategy/tests/latency_slippage_sim_e2e.rs` — v5 itself (Wave A re-check)

TCN / PatchTST overlays do not currently carry a dedicated
`*_end_to_end.rs` divergence test (their alpha is captured by anchored
backtest reports; covered by Wave A re-emission, not Wave D).
Architect re-checks this list at the start of Wave D.

**Why re-run all, not just load-bearing**: anchor cascade isn't
optional. Half-migrating leaves silent invariant drift in
`crates/strategy/tests/`. Cost is ~2-3 dev-days vs ~6 weeks of
accumulated drift if deferred.

### D5 — Sharpe-delta regression gate is a permanent contract

The dev-note authored by developer Wave C (per R5) is **load-bearing
forever**: every future change to an anchored strategy must re-render
this table and re-justify any movement in the "Flipped?" column.

This makes the noop-baseline namespace operationally meaningful in
perpetuity, not just at v0.2.0 ship. The contract is:

- A scenario whose `sharpe(noop) - sharpe(canonical)` widens by more
  than the v0.2.0-locked threshold (placeholder: 0.5 Sharpe units —
  developer / tester confirm at Wave C) → triggers an explicit ADR
  amendment or follow-on brief.
- Alpha flipped sign → mandatory operator review per D3.

This contract lives in `spec/architecture.md` § Regression gate
discipline (developer adds the cross-reference at Wave B per
[`spec-update`](../../.claude/skills/spec-update/SKILL.md)).

## Alternatives considered

- **Single-namespace, retire OLD anchors (Q2 = (b))** — rejected.
  Loses the friction-free oracle. The CLAUDE.md ≥ 1 bp non-negotiable
  needs a noop reference; without it the assertion has no anchor.
- **Both visible at same version pin (Q2 = (c))** — rejected.
  Ambiguous which row `verify_anchors.sh` consumes; would require
  scenario-name suffix gymnastics. Namespace separation is cleaner.
- **Ship 3 namespaces (tight + medium + aggressive ladder, Q1 = (d))**
  — rejected. Triples cost; dilutes "canonical" semantics. Stress
  config will arrive in its own namespace at v0.3.0 if needed.
- **Auto-retire inverted scenarios (Q3 = (a))** — rejected.
  Irreversible without re-spec. Loses operator judgment.
- **Defer cross-feature re-check (Q4 = (c))** — rejected. Accumulates
  silent overlay invariant drift over weeks.

## Consequences

### Positive

1. **Every alpha number in `spec/anchors.toml` now represents
   strategy edge UNDER realistic friction.** The backtest-vs-live gap
   closes by construction, not by virtue of having the engine
   compiled in.
2. **Permanent regression gate via Sharpe-delta table.** Any future
   strategy work re-renders the table; alpha drift becomes
   immediately surfaced.
3. **Noop-baseline namespace is the friction-free oracle forever** —
   protects the CLAUDE.md ≥ 1 bp non-negotiable from semantic decay.
4. **Operator-judgment trail on K1 surprises** — per-scenario flag +
   explicit retire / accept / refine decision per inverted scenario.

### Negative

1. **`spec/anchors.toml` doubles in row count** from 34 to 68. File
   size and review overhead grow proportionally. Mitigated by
   namespace-grouped comments + a header sentence per D2.
2. **Strategy alpha disappears for some scenarios under 8 bps
   slippage.** H1 hypothesises ≤ 3 scenarios flip; if > 3, the brief
   routes to R-O3 (operator-decide refine vs accept).
3. **Cross-feature re-run cost** ~2-3 dev-days at Wave D. Budgeted
   per feature.md § Cost framing.
4. **Operator dashboard / CI gates expecting OLD SHAs would break.**
   Survey at M-T1: only `scripts/verify_anchors.sh` consumes the file;
   it reads structurally, not by SHA literal. K3 monitored.

## Cross-references

- v0.2.0 feature brief — [`spec/v5-latency-slippage-sim-v0.2.0-anchor-migration/feature.md`](../../v5-latency-slippage-sim-v0.2.0-anchor-migration/feature.md)
- v0.2.0 tasks — [`spec/v5-latency-slippage-sim-v0.2.0-anchor-migration/tasks.md`](../../v5-latency-slippage-sim-v0.2.0-anchor-migration/tasks.md)
- Parent ADR — [`ADR-0043 simulated-latency-and-slippage`](0043-simulated-latency-and-slippage.md)
- Anchor-additive precedent — [`ADR-0038 spec-anchor-bounded-set-discipline`](0038-vol-forecast-verdict-shape.md)
- Trace row — `REQ-V5-ANCHOR-MIGRATION-V0-2-0-001` in [`spec/trace.toml`](../../trace.toml)
- Anchors file (target of migration) — [`spec/anchors.toml`](../../anchors.toml)
- CLAUDE.md non-negotiable — `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs` pattern reference
- Verify script — `scripts/verify_anchors.sh`
- Hash recompute — `scripts/hash_report.py`

## Changelog

- 2026-05-27 (architect M-T1): ADR-0045 authored. 5 sub-decisions
  (D1-D5) locked from operator M-OD resolutions Q1=(b) medium /
  Q2=(a) noop-baseline namespace / Q3=(b) per-scenario flag /
  Q4=(a) re-run all overlay e2e. Canonical namespace pin =
  `v5-realdata-medium-2026-05`. Sharpe-delta table promoted to
  permanent regression-gate contract per D5.
