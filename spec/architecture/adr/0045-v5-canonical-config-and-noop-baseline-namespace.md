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

### D6 — The two anchor systems are RECONCILED, never independently authored (amendment, 2026-05-30)

> Adopted after the engine-drift PAPERWORK finding
> ([`spec/dev-notes/engine-drift-diagnosis-2026-05-30.md`](../../dev-notes/archive/2026-Q2/engine-drift-diagnosis-2026-05-30.md),
> diagnosis commit `1cbe3d4`). It closes a gap D2 created but did not
> name: there are **two** physical anchor systems, and D2 only governed
> one of them.

This repo carries **two** body-SHA-256 regression systems for the same
engine output. They had silently diverged for ~3 months (the
`Q-D1=(a)` 0→8 bps synthetic-slippage change at `7e8a7e0`, 2026-05-29,
moved the engine output; only system (1) was updated):

1. **FILE-anchors** — `spec/anchors.toml`, hashed by
   `scripts/verify_anchors.sh` against **saved** `.md` report bodies on
   disk. This system is governed by D1-D5 + the ADR-0038 § D6 / § D6.b
   contract. It **cannot detect engine-behaviour drift** — it hashes
   the file an earlier run wrote, not what the current binary emits.
2. **IN-TEST re-run anchors** — `const ANCHOR` / `const ANCHOR_PREFIX`
   strings in `crates/backtest/tests/determinism.rs` (the `t622_*` /
   `t717_*` / `tt1_*` `*_anchor_hash_unchanged` tests). These **re-run
   the engine** in-process and compare the freshly-emitted body-SHA to
   a hardcoded constant. This system **does** detect engine drift — but
   it has no auto-link to system (1) and rotted silently.

**Decision (D6.1) — single source of truth.** `spec/anchors.toml` is
the **canonical** anchor registry. The `determinism.rs` constants are a
**derived projection** of it for the synthetic, non-feature-gated
scenarios. They are never hand-authored against a fresh run; they are
re-locked **to the matching `spec/anchors.toml` SHA** for the scenario
under the namespace the default (no-feature) `cargo test` binary
produces. Mapping rule:

- The `determinism.rs` non-feature-gated tests build the binary with
  **no `realdata` / `candle` feature**. Under
  `build_slippage_model_for_scenario` (`crates/backtest/src/main.rs`)
  every scenario NOT in `REAL_DATA_SCENARIO_IDS` (and ALL scenarios when
  `realdata` is absent) takes the `Linear { bps: 8 }` synthetic
  fallback. Therefore the in-test constant for scenario *S* MUST equal
  the `spec/anchors.toml` row `scenario = S, version = "… + v5-realdata-medium-2026-05"`
  — the canonical-friction SHA, NOT the `noop-baseline` SHA.
- A `noop-baseline` SHA may only appear in `determinism.rs` if a test
  is explicitly run with friction forced to zero (none are today).

**Decision (D6.2) — reconciliation, not a new anchor decision.**
Updating a `determinism.rs` constant from a stale `noop-baseline` SHA to
the already-committed `v5-realdata-medium-2026-05` SHA is a
**reconciliation of a derived projection to its already-approved
source**, not a new anchor lock. It does NOT require the ADR-0038 § D6.b
5-step re-emission protocol (no saved file mutates; the canonical
file-anchor was operator-ratified at `7e8a7e0`). It DOES require
architect sign-off as a regression-gate edit per the `spec/anchors.toml`
owner policy — granted here for the 14-test reconciliation enumerated in
the engine-drift dev spec.

### D7 — Closing the engine-drift blind-spot (amendment, 2026-05-30)

**Problem.** `verify_anchors.sh` is the gate the tester is told to run
before `VERDICT → PASS` (`.claude/skills/verify-anchors/SKILL.md`); it
hashes saved files and so is structurally blind to engine-behaviour
drift. The in-test re-run gate that *would* catch it
(`determinism.rs`) is part of `cargo test --workspace --all-targets`
(it is NOT `#[ignore]`d) — but there is **no CI** (`no .github/workflows`)
and the `rust-validate` gate runs fmt/clippy/audit/deny/doc only, not
tests. A ship that ran the realdata backtest sweep + `verify_anchors.sh`
but not the full synthetic determinism suite let the drift through.

**Decision — synthesis of options (b) + (a), in that priority.**

- **D7.1 (primary, option b) — mechanical auto-sync + assert.** Add
  `scripts/check_determinism_anchors.py` (architect-specced; developer
  implements). It parses `spec/anchors.toml` and the
  `const ANCHOR`/`ANCHOR_PREFIX` sites in `determinism.rs`, builds the
  scenario→SHA map for the non-feature-gated synthetic tests, and
  **asserts each in-test constant equals the matching
  `v5-realdata-medium-2026-05` row** (per D6.1). On mismatch it exits 1
  and prints a drift table (scenario, in-test value, anchors.toml value,
  file:line). A `--write` mode rewrites the constants in place so the
  two systems can never be reconciled by hand-typing a SHA. This is the
  durable fix: it converts a "someone must remember to re-run the slow
  re-run suite" problem into a sub-second static check with no engine
  execution. Mirrors the existing `scripts/adr_registry_check.py`
  drift-linter pattern.

- **D7.2 (secondary, option a) — enforce the re-run suite at the gate
  the tester actually runs.** The `verify-anchors` skill is the
  documented pre-`VERDICT → PASS` gate for any change touching
  strategy / audit / exec / backtest code. Amend that skill to ALSO run
  the synthetic determinism re-run tests in **release** mode after
  `verify_anchors.sh` passes:
  `cargo test --release -p backtest --test determinism t622_ t717_ tt1_`.
  Release mode is mandatory for cost (see below). This is the
  belt-and-suspenders catch: D7.1 detects *constant-vs-file* drift
  statically; D7.2 detects *engine-vs-constant* drift (e.g. a code
  change that moves output AND the dev forgot to re-lock) by actually
  re-running.

- **D7.3 (option d, partial) — document the dual model; do NOT retire
  `noop-baseline`.** The `noop-baseline` rows stay (D2 already makes
  them the friction-free oracle for the CLAUDE.md ≥ 1 bp non-negotiable).
  The two-system model and the D6.1 mapping rule are documented in
  `spec/anchors.toml`'s header and `spec/architecture.md` § Regression
  gate discipline so the next author understands *why* there are two
  systems and *which* is canonical. Option (c) (teach
  `verify_anchors.sh` to regenerate-and-hash) is **rejected**: it would
  fold engine execution into a script whose entire design point is
  cheap file-hashing, and it duplicates what `determinism.rs` already
  does correctly.

**Cost (flagged for operator).** D7.1 is ~sub-second, zero engine runs —
free. D7.2 re-runs ≈14 synthetic scenario invocations. Measured on the
canonical Apple-Silicon box, **debug** binary: sma-family ≈ 8.8 s/run,
momentum ≈ 26 s/run → the full in-test set is multi-minute in debug.
**Release** binary amortises this to well under a minute total after the
one-time build. The recommendation runs D7.2 in **release only**, and
only inside the `verify-anchors` gate (which already gates on
backtest-touching changes) — NOT on every `cargo test`. Net added
wall-clock on a backtest-touching ship: < ~1 min (release) on top of an
already-required backtest sweep. If even that is unwanted, D7.1 alone
still makes the *specific* drift that occurred here impossible to ship
silently; D7.2 additionally guards the harder "engine moved + nobody
re-locked anchors.toml either" case. Operator may down-scope to D7.1-only
if the < 1 min release cost is judged not worth it.

### D6.3 — The `v5-realdata-medium-2026-05` namespace is provenance-MIXED (amendment, 2026-05-30, second pass)

> Adopted after a developer correctly STOPPED at the engine-drift fix
> verification step (VR-1): 6 of the 14 in-test constants
> (macd/rsi/bbands × t622+t717) did NOT reproduce their EX-1-mapped
> `v5-realdata-medium-2026-05` SHA when re-run. Architect re-verified the
> root cause; it refines D6.1.

D6.1 stated the mapping rule "in-test constant for scenario *S* == the
`v5-realdata-medium-2026-05` `anchors.toml` row for *S*" as if the no-feature
default binary reproduces those SHAs for **every** scenario. **It does not.**
The `v5-realdata-medium-2026-05` namespace is **provenance-mixed** — the saved
report files it anchors were NOT all emitted from the synthetic path:

| Scenario group | `data_source` of the saved v5 file | Bars | In-test (synthetic) SHA |
|----------------|------------------------------------|------|--------------------------|
| `btc-2023-1m-sma-cross`, `-sma-baseline-refresh` | `synthetic (v0 fallback)` | 525601 | **==** the v5 row (`d2fa7616…`) |
| `top10-{2023,2024}…momentum` | `synthetic (v1 multi-symbol)` | — | **==** the v5 row (`0f6f6eb8…`/`78976062…`) |
| `top10-{2023,2024}-fy-tcn-overlay` (tt1) | synthetic (v2.5 passthrough) | — | **==** the v5 row (`1460fcc7…`/`b8e9186b…`) |
| **`btc-2023-1m-{macd-trend,rsi-reversion,bbands-mean-revert}`** | **`real (Binance Vision)`** | **17544** | **≠** the v5 row — synthetic SHA is `4d8192af…`/`4a744788…`/`5037accb…`, held in D7.1's override map |

**Root cause.** At the v0.3.0 re-emission (commit `21bda41`, 2026-05-27) the
emitting machine had `btc-2023-1m` Binance parquet on disk. The SMA/Composed
group was run `--force-synthetic-bars` (Q1=(a) revert; see `anchors.toml`
header line 330), but the macd/rsi/bbands scenarios were NOT forced and so took
the **real-data** path (17544-bar bodies). The `anchors.toml` header's "Group A
→ synthetic" line was over-broad and silently mis-stated those 3. The header is
corrected alongside this amendment.

**Why the determinism.rs constant is the SYNTHETIC SHA regardless.** The
`determinism.rs` re-run tests (`run_scenario_once`) spawn the binary with
`.current_dir(tempdir)`. The binary resolves Binance parquet **relative to
CWD**; in a tempdir the lookup always misses → the engine takes the v0
synthetic fallback for ALL scenarios, on every machine, deterministically.
These tests are by-design **pure synthetic regression guards**; their JOB is to
lock the deterministic synthetic engine path. So the correct in-test constant
for macd/rsi/bbands is the **synthetic** body-SHA — which has no `anchors.toml`
row.

**Refined mapping rule (supersedes the single clause in D6.1).** The in-test
constant for a non-feature-gated scenario *S* equals, in order: (1) the
synthetic body-SHA recorded in D7.1's `SYNTHETIC_DETERMINISM_SHAS` map if *S* is
listed there (the scenarios whose `v5-realdata-medium` row is real-data
provenance); else (2) the `v5-realdata-medium-2026-05` `anchors.toml` row for
*S* (the scenarios whose row is synthetic provenance — sma/momentum/tt1). Both
cases assert the engine's **synthetic-path** output; they differ only in WHERE
the authoritative synthetic SHA is stored, because `anchors.toml` happens to
carry a synthetic SHA for one group and a real-data SHA for the other.

### D7.1b — Synthetic-override map, NOT new `anchors.toml` rows (amendment, 2026-05-30, second pass)

The blocker raised three options for making D7.1 GREEN for the 6 mixed-provenance
scenarios. **Decision: option 2a-refined (synthetic-override map inside D7.1).
Option 2b (add synthetic `anchors.toml` rows) is REJECTED.**

- **2a-refined (CHOSEN).** D7.1 (`check_determinism_anchors.py`) carries an
  explicit `SYNTHETIC_DETERMINISM_SHAS` dict (the 3 scenario→synthetic-SHA
  entries). Resolution order per in-test site: synthetic-override map → then
  `v5-realdata-medium-2026-05` row → else **HARD ERROR** (a scenario in neither
  map is a linter failure, not a silent skip). This keeps D7.1's invariant
  uniform and total: every non-cfg-gated in-test constant has exactly one
  authoritative source and any drift fails the linter. The map lives adjacent
  to the constants it guards, so re-locking a synthetic constant and updating
  its map entry is a single-file edit.

- **2b (REJECTED) — add synthetic rows to `anchors.toml`.** `verify_anchors.sh`
  requires **every** `[[anchors]]` row to resolve to a saved `.md` report on
  disk whose body hashes to the row's SHA (it keys file-lookup on the `version`
  namespace and emits `MISS`+fail for any unresolvable row). No saved synthetic
  report exists for macd/rsi/bbands (the only saved v5 files are the 17544-bar
  real-data ones). 2b would therefore force EITHER committing 3 new synthetic
  report files — which become byte-immutable anchored artifacts forever (ADR-0038
  § D6), plus a new `verify_anchors.sh` namespace branch, plus the row count
  86→89 — OR leaving 3 unresolvable rows that break the file-anchor gate. Both
  are disproportionate. The in-test re-run gate already IS the synthetic
  regression guard for these scenarios; a redundant file-anchor adds immutable
  surface and a second source of truth for no marginal coverage.

- **2c (REJECTED) — exempt the 6 from D7.1 entirely.** This reopens the exact
  silent-drift blind spot D7 exists to close: an un-checked constant could rot
  unnoticed. "Exempt from the `anchors.toml` check" must mean "checked against a
  different explicit source," never "unchecked." The HARD-ERROR-on-unmapped rule
  in 2a-refined is what makes the exemption safe.

**Residual cost (flagged).** The 3 synthetic SHAs are not in `anchors.toml`;
their source of truth is the D7.1 map. If a future engine change moves the
synthetic macd/rsi/bbands output, D7.2 (re-run) catches it, and the dev re-locks
BOTH the `determinism.rs` constant AND the map entry in one edit. This is a
bounded, documented cost accepted in exchange for not growing the immutable
file-anchor set.

## Alternatives considered

- **Add synthetic `anchors.toml` rows for macd/rsi/bbands (engine-drift 2b)**
  — rejected; see § D7.1b. Would force 3 new byte-immutable report files or
  break `verify_anchors.sh`, for zero coverage the in-test gate doesn't already
  provide.
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
- Engine-drift diagnosis (D6 / D7 trigger) — [`spec/dev-notes/engine-drift-diagnosis-2026-05-30.md`](../../dev-notes/archive/2026-Q2/engine-drift-diagnosis-2026-05-30.md) (commit `1cbe3d4`)
- In-test re-run anchor gate — `crates/backtest/tests/determinism.rs` (`t622_*` / `t717_*` / `tt1_*`)
- D7.1 drift-linter — `scripts/check_determinism_anchors.py` (implemented 2026-05-30; pattern: `scripts/adr_registry_check.py`; § D7.1b adds the `SYNTHETIC_DETERMINISM_SHAS` override map for the 6 mixed-provenance constants)
- Engine-drift fix dev spec (EX-1 corrected rows + BLOCKER resolution + EX-4 v2) — [`spec/dev-notes/engine-drift-fix-spec-2026-05-30.md`](../../dev-notes/archive/2026-Q2/engine-drift-fix-spec-2026-05-30.md)

## Changelog

- 2026-05-27 (architect M-T1): ADR-0045 authored. 5 sub-decisions
  (D1-D5) locked from operator M-OD resolutions Q1=(b) medium /
  Q2=(a) noop-baseline namespace / Q3=(b) per-scenario flag /
  Q4=(a) re-run all overlay e2e. Canonical namespace pin =
  `v5-realdata-medium-2026-05`. Sharpe-delta table promoted to
  permanent regression-gate contract per D5.
- 2026-05-30 (architect): § D6 + § D7 amendment, triggered by the
  engine-drift PAPERWORK finding (diagnosis `1cbe3d4`). D6 names the
  two-anchor-system reality D2 left implicit and makes
  `spec/anchors.toml` the single source of truth, with the
  `determinism.rs` constants a derived projection re-locked to the
  `v5-realdata-medium-2026-05` SHAs (NOT `noop-baseline`) for the
  no-feature default test binary; reconciling them is NOT a § D6.b
  re-emission. D7 closes the blind-spot: D7.1 a sub-second
  `check_determinism_anchors.py` static drift-linter (+ `--write`
  auto-sync), D7.2 enforce the synthetic re-run determinism tests
  (release mode) inside the `verify-anchors` gate, D7.3 document the
  dual model + keep `noop-baseline`. Rejected option (c)
  (regenerate-and-hash inside `verify_anchors.sh`). Operator cost flag:
  D7.2 adds < ~1 min release wall-clock on backtest-touching ships;
  down-scope to D7.1-only is available.
- 2026-05-30 (architect, second pass): § D6.3 + § D7.1b added after a
  developer correctly STOPPED at engine-drift VR-1. The
  `v5-realdata-medium-2026-05` namespace is provenance-MIXED:
  `btc-2023-1m-{macd-trend,rsi-reversion,bbands-mean-revert}` v5 rows were
  emitted from the REAL-DATA path (17544-bar Binance bodies), while
  sma/momentum/tt1 v5 rows are synthetic. The `determinism.rs` tests run the
  synthetic v0-fallback path (CWD=tempdir), so the 6 macd/rsi/bbands in-test
  constants are the SYNTHETIC SHAs (`4d8192af…`/`4a744788…`/`5037accb…`), held
  in D7.1's `SYNTHETIC_DETERMINISM_SHAS` override map — NOT in `anchors.toml`.
  D6.3 refines the D6.1 mapping rule (synthetic-override → canonical → HARD
  ERROR). D7.1b rejects adding synthetic `anchors.toml` rows (option 2b: would
  break `verify_anchors.sh` or grow the immutable report-anchor set for zero
  marginal coverage) and rejects exempting the 6 (2c: reopens the blind spot).
  No `anchors.toml` row or saved report file changes; VR-3 stays 86/86.
