---
slug: testing-strategy-review-2026-05-25
status: review
owner: analyst
updated: 2026-05-25
---

# Testing strategy review (2026-05-25)

This is a **product-fit review of the workspace's testing approach**, not a
tooling survey. The orchestrator's parallel architect thread is running
[`testing-framework-audit-2026-05-25`](testing-framework-audit-2026-05-25.md)
to pick the technical topology (cargo-llvm-cov vs tarpaulin, CI shape,
etc.). My scope is the strategic question the operator asked on 2026-05-25:

> Are we testing the **right things** for what this project IS, given (1)
> [`spec/product.md`](../product.md)'s "auditable + persistent reflection
> memory + safe-by-typed-risk" goals and (2) the operator's reframed goal
> locked 2026-05-16: *"real, working, auditable agent architecture; operator
> learns by building it"*?

The trigger is the **Bug #63 Lab progress-bar incident** ([`spec/bug-log.md`
\#63](../bug-log.md)) — the cross-sectional Stop button and progress wiring
shipped dead, with green math-layer tests and byte-identical anchored
reports. Same shape as the **v3-volatility-forecaster no-op discovery**
(2026-05-22, [`docs/dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md`](v3-vol-overlay-noop-discovery-2026-05-22.md)):
unit tests on `compute_scale` math passed; anchored backtest reports
hashed byte-identically; the overlay's `scale` was computed and
discarded. Pattern, not a one-off.

My POV: **the workspace tests a lot, and tests well at the math layer,
but does not test the surfaces this project's product goals make
load-bearing**. The "auditable" goal demands wire-completeness tests
(every overlay actually applies); the "operator learns by building"
goal demands tests that fail in operator-readable ways (and that exist
*because they teach a failure mode*, not because they hit a coverage
threshold); the "real working agent" goal demands operator-workflow
e2e coverage that the suite barely has. The Lab progress-bar bug
exists because none of those properties have a gate.

## §1 — Product-test alignment grades

The properties the product demands, drawn from
[`product.md`](../product.md) and the operator's 2026-05-16 reframe,
graded against the current testing layers. Per-property A-F + evidence.

### P1 — Auditability (double-entry ledger, every decision reconciles)

> Source: [`product.md` ## Goals](../product.md), Differentiator (4),
> [`architecture.md` § Cross-cutting invariants](../architecture.md#cross-cutting-invariants)
> rule 1.

**Grade: B+**

Strong here. The cross-cutting invariant — "audit imports nothing from
sibling crates" — is enforced by Rust's dependency graph itself
(`crates/audit/Cargo.toml` cannot grow a sibling dep without breaking
`cargo check`). The reconciler test
`crates/audit/tests/reconciler.rs` (`../../crates/audit/tests/reconciler.rs`)
asserts `Σ debits == Σ credits` after each fixture migration. The
6-digit fractional-second timestamp invariant
([ADR-0004](../architecture/adr/0004-fractional-second-timestamps.md))
has both a unit test (`audit::ts::roundtrip_micros`) and acts as a
de-facto gate via the 11-anchor body-SHA gate
([`architecture/11-regression-gate.md`](../architecture/11-regression-gate.md)).

What stops it being A: there is **no end-to-end ledger-reconciliation
property test** that drives a randomised stream of fills through the
ledger and asserts reconciliation invariants hold. The closest is
`crates/audit/tests/reconciler.rs` which is example-based on three
hand-built fixtures. The audit layer is the project's claimed moat;
its tests should be the most paranoid in the workspace, and right
now they are not.

### P2 — Wire-completeness ("the overlay actually applies")

> Source: [`CLAUDE.md` ## Non-negotiables](../../CLAUDE.md#non-negotiables) —
> "Every strategy overlay or sizing-modifier ships with a
> baseline-equity-divergence end-to-end test from day 1." Pattern
> reference:
> [`crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`](../../crates/strategy/tests/vol_targeting_overlay_end_to_end.rs).

**Grade: D**

Documented; not enforced. Mechanical evidence:

```
crates/strategy/src/                         ←→  crates/strategy/tests/
  patchtst_overlay_momentum.rs                   (no *_end_to_end.rs)
  tcn_overlay_momentum.rs                        tcn_overlay_tuned_builder.rs   ← unit only
  vol_killswitch_overlay.rs                      (no *_end_to_end.rs)
  vol_targeting_overlay.rs                       vol_targeting_overlay_end_to_end.rs   ✓
```

One of four overlays has the mandated e2e divergence test. The other
three are wire-bug-bait. `vol_killswitch_overlay.rs` is the scariest
gap because **a killswitch that doesn't kill is the worst kind of no-op**
— silent under the same conditions that caused the v3 vol-target
no-op (math computed, application never wired). The TCN overlay's
`tcn_overlay_tuned_builder.rs` is a builder unit test, not an
equity-divergence guard. The PatchTST overlay shipped Wave D 2026-05-22
with the same risk profile.

The rule was added to `CLAUDE.md` *three days ago* (2026-05-22) after
discovering the v3 no-op. The fact that none of the three pre-existing
overlay siblings has been retrofitted is the evidence that the rule
is a wish, not a gate. See §2 for the enforcement proposal.

### P3 — Persistent reflection memory (the second moat)

> Source: [`product.md`](../product.md) Differentiator (2),
> [`spec/architecture/05-llm-and-reflection.md`](../architecture/05-llm-and-reflection.md).

**Grade: B**

The reflection crate has 10 test files
(`crates/reflection/tests/`) covering store smoke, top-K determinism,
idempotency, embedding determinism (proptest), post-mortem card
generation, back-pressure, and — notably — the **`no_strategy_caller.rs`
hygiene grep** that fails CI if the strategy crate ever imports
`reflection::retrieve_top_k` without a follow-up `reflection-memory-trader-wiring`
brief. That hygiene test is the **template I want to copy for §2**.

What stops it being A: there is no end-to-end "lesson card written →
trader retrieves it on next decision → outcome changes" integration
test. The reflection loop is the operator's "is the agent learning?"
moment; today it has unit-tested components but no operator-readable
gate that proves the *loop* runs. This will land naturally with the
queued `reflection-memory-trader-wiring` feature but it is a gap
*right now*.

### P4 — Operator workflow integrity (the Lab incident class)

> Source: 2026-05-16 reframe — *"real, working, auditable agent
> architecture; operator learns by building it"*. The Lab is the
> operator's primary learning loop.

**Grade: D−**

This is where the workspace bleeds. The cockpit_live binary is the
operator's daily surface; it has **zero automated smoke that drives
it through a Lab Run**. The 9 cockpit-smoke log files under various
`spec/<slug>/reports/` are fixtures-mode runs (the cockpit-smoke skill
itself is fixtures-bound by design, see
[`.claude/skills/cockpit-smoke/`](../../.claude/skills/cockpit-smoke)).
That is correct hygiene given the agent sandbox boundary — fixtures
mode is deterministic — but it leaves the **live binary** running on
a wing and a prayer.

Bug #63's signature: progress-bar wiring missing for cross-sectional
scenarios, Stop button silent. Both are surfaces the operator hits
*on every Lab Run*; both shipped because no test drives the
`crates/ui/src/lab/runner.rs` path through a real scenario with a
live `cancel_rx` / `progress_tx`. The math layer was fine. The
anchored reports byte-matched (because `cancellation_pair()` and
`ProgressSender::disabled()` are passed by the CLI, exactly to keep
backtest output stable — clever, but it means the live UI path
NEVER runs in CI). The bug ships.

There ARE 13+ integration tests in `crates/ui/tests/` — `lab_run_*.rs`,
`cockpit_live_kill_button_writes_audit.rs`, etc. — that drive `update`
through messages. But none of them exercise the cross-sectional
scenario branch with progress polling on a 128-bar boundary. The
gap is precise and structural: the e2e tests cover the *happy path
of the strategies that already had tests*, not the *whole grid of
strategy × UI surface*.

The 2026-05-16 reframe says "operator learns by building it" — but
right now the operator learns by **shipping bugs**, because the test
suite doesn't cover the surfaces the operator touches. That's not
"learning by building"; it's "discovering by breaking." Different
thing.

### P5 — Auditable report determinism (the anchor gate)

> Source: [`architecture/11-regression-gate.md`](../architecture/11-regression-gate.md),
> [`anchors.toml`](../anchors.toml) (301 lines, 11+ scenarios).

**Grade: A−**

Genuine strength. 49 anchored backtest reports across 22 scenarios,
byte-SHA-256 over the body, frontmatter excludes run-varying values,
the verify-anchors skill is mechanical and the ADR-0038 § D6 re-emission
protocol gates legitimate mutations. The body-vs-frontmatter discipline
caught a real regression (HF-1, 2026-04-18) and now self-enforces.

What stops it being A: the gate's failure mode under the v3 no-op was
to be **a useful witness but not an interpreter**. Two genuinely
different runs (overlay-on, overlay-off) producing byte-identical
output is the no-op's exact signature, and the gate **passed** —
because passing is what byte-identity means under its rules. The
gate is correctly tuned for "the output didn't drift" but it cannot
also answer "the output IS different from a meaningfully-different
neighbour." That's a higher-order property and needs the e2e
divergence test (P2 above) as its complement. The gate is not
broken; it's just doing only the job it was scoped to do.

### P6 — Risk-engine safety (typed-illegal-orders, hard limits)

> Source: [`product.md` ## Risk management](../product.md#risk-management-hard-requirements),
> [`spec/architecture/04-risk-and-money.md`](../architecture/04-risk-and-money.md).

**Grade: B**

The `Money<C: Currency>` newtype + `rust_decimal::Decimal` invariant
([ADR-0003](../architecture/adr/0003-decimal-money-math.md)) enforces
exact-cent math at the type level, which is the strongest possible
gate (compile-time). `Quantity::new`, `Price::new`, and `Order`
constructors all return `Result`, so illegal orders fail at
construction. The risk crate ships proptest invariants
(`crates/risk/src/portfolio.rs`).

What stops it being A: the **kill switch's end-to-end behavior** is
the project's hardest safety claim and it has one test
(`crates/ui/tests/cockpit_live_kill_button_writes_audit.rs`) plus a
manual operator verification at v0 ship. For a property the product
calls "the only path to live exchange" and "presence of `.halt` file
→ flatten and stop," I would expect a dedicated chaos-style test
that fuzzes order arrival around `.halt` creation and asserts
*no order lands post-halt*. That test does not exist. Property
test exists for portfolio invariants; the kill switch is example-only.

### P7 — Operator-readable failure modes ("learns by building")

> Source: 2026-05-16 reframe.

**Grade: C**

When a test fails, does the operator learn what went wrong from the
test message alone? Mixed:

- The vol_targeting_overlay e2e test
  ([`vol_targeting_overlay_end_to_end.rs:160-166`](../../crates/strategy/tests/vol_targeting_overlay_end_to_end.rs))
  asserts with `'vol-target overlay produced byte-identical equity
  to baseline — no-op suspected'`. That message **teaches** —
  operator reads it, immediately knows the failure class.
- `cargo test reconciler` failing surfaces `assertion failed: a == b`
  with two Decimal values and zero context.
- `verify_anchors.sh` failing dumps a hash diff with no explanation
  of *why* the body might have changed.
- proptest failures shrink to a minimal case, which is good, but
  the assertion text is usually mechanical (`prop_assert_eq!`),
  not pedagogical.

The asymmetry: tests that were authored after a bug post-mortem
(vol_targeting_overlay e2e) carry the lesson. Tests authored at
green-field implementation time don't. Since this project values
operator learning, **every test message ought to include the
failure-mode hint**, not just the assertion. That's a documentation
debt, not a structural one — see §5 Q5 below.

### Grade summary

| Property | Grade | Mode |
|---|---|---|
| P1 — Auditability (double-entry) | **B+** | Strong; missing reconciler property test |
| P2 — Wire-completeness (overlays apply) | **D** | Rule documented, not enforced; 1/4 overlays covered |
| P3 — Reflection memory | **B** | Components tested; loop not e2e-tested |
| P4 — Operator workflow integrity | **D−** | Live binary has no smoke; Bug #63 is the witness |
| P5 — Report determinism (anchor gate) | **A−** | Mechanical, ratified, but cannot detect "no-op produces baseline" |
| P6 — Risk-engine safety | **B** | Type-level strong; kill-switch e2e is example-only |
| P7 — Operator-readable failure modes | **C** | Hit-and-miss; depends on whether test was bug-driven |

The top-3 grades by *gap* are **P2 (D)**, **P4 (D−)**, **P7 (C)**.
P5 is genuinely strong and the operator should be proud of it. P1
and P3 are solid; P6 is solid where it counts (type-level) and weak
where it matters most (e2e kill-switch).

## §2 — The no-op pattern enforcement gap

> Question: is the CLAUDE.md non-negotiable
> ("every overlay/sizing-modifier ships with a baseline-equity-
> divergence e2e test from day 1") **enforced or documented**?

**Documented, not enforced.** Evidence enumerated in P2 above:
3 of 4 overlay files in `crates/strategy/src/` have no matching
`*_end_to_end.rs` file. The vol_killswitch_overlay (the most
safety-critical of the three) has zero tests of any kind in
`crates/strategy/tests/` (no file with `killswitch` in its name).

### Proposed enforcement — a hygiene test

The pattern already exists in this repo:
[`crates/reflection/tests/no_strategy_caller.rs`](../../crates/reflection/tests/no_strategy_caller.rs)
walks `crates/strategy/src/` looking for forbidden substrings and
fails the build when one appears. Same shape, inverted polarity:

```rust
// crates/strategy/tests/overlay_e2e_coverage.rs (PROPOSED)
//
// Scans crates/strategy/src/ for files matching `*_overlay*.rs`
// and asserts each has a matching `crates/strategy/tests/<stem>_end_to_end.rs`.
// FAILS the build when a new overlay lands without its day-1 e2e divergence test.

#[test]
fn every_overlay_has_an_end_to_end_test() {
    let src = workspace_path("crates/strategy/src");
    let tests = workspace_path("crates/strategy/tests");
    let mut missing: Vec<String> = Vec::new();
    for entry in walkdir::WalkDir::new(&src) {
        let p = entry?.path();
        let name = p.file_stem().and_then(|s| s.to_str())?;
        // Match files that look like overlay implementations.
        // Heuristic: contains "overlay" and is not "lib.rs", "mod.rs", "traits.rs".
        if !name.contains("overlay") { continue; }
        let expected = tests.join(format!("{name}_end_to_end.rs"));
        if !expected.exists() {
            missing.push(format!(
                "overlay {p:?} has no {expected:?} — see CLAUDE.md ## Non-negotiables \
                 and crates/strategy/tests/vol_targeting_overlay_end_to_end.rs \
                 for the pattern. To suppress: ADD an opt-out attribute (see below)."
            ));
        }
    }
    assert!(missing.is_empty(), "overlay e2e coverage gap:\n{missing:#?}");
}
```

**The opt-out attribute** matters: some overlay files may legitimately
not be reachable as a runtime overlay (e.g. a builder helper that the
type system disguises as an `*_overlay*.rs` file by naming convention).
The opt-out is a `// ALLOW_NO_E2E: <reason>` comment at the top of the
file with a SHA pin so it can't be added without a code review noticing.

**Cost**: ~30 LOC, half a day. Closes the entire P2 gap mechanically.
The architect's parallel
[`testing-framework-audit-2026-05-25`](testing-framework-audit-2026-05-25.md)
should consider whether to put this in `crates/strategy/tests/`,
`crates/reflection/tests/` (the no_strategy_caller pattern lives
there), or a new `crates/hygiene/` crate that owns the project's
defensive-grep tests as a category.

**Retrofit**: shipping the hygiene test today fails CI for three
overlays. Operator-decide: ship the gate and accept that those three
fail until their e2e tests land (forcing function), or ship the gate
in `#[ignore]` mode for 1 week to give a window for retrofit. My
recommended default is **fail loud immediately** — the rule was
authored *because* the v3 no-op already shipped; carrying the debt
in a quiet `#[ignore]` perpetuates the exact failure mode the rule
exists to prevent.

**Why this is not "just add more tests"**: it is a *forcing function*.
The operator's 2026-05-16 reframe is "operator learns by building";
a hygiene gate makes the lesson structural. The next overlay author
(human or agent) cannot ship without writing the e2e test, and the
e2e test exists at the layer where the bug manifests, not at the
math layer where it's invisible.

## §3 — Coverage in this project's context

Coverage as a metric is famously gameable. For *this* project,
specifically, my POV is below.

### High-value lines vs low-value lines

The lines that EARN their coverage:

- **Strategy `on_bar` / `on_signal` paths** — every Signal arm,
  every overlay application, every sizing branch. A surviving
  mutant here is a no-op-pattern bug waiting.
- **Fill logic in `crates/exec/`** — fee math, slippage application,
  partial-fill state machines. Money-math layer.
- **Audit ledger writes** — `audit::write_decision`, `audit::write_fill`,
  reconciler glue. Anything where a divergent state is silently
  acceptable.
- **Reflection memory `store::write` / `retrieve_top_k`** — the moat.
- **LLM tool dispatch in `crates/llm/`** — the schema-validation
  branches; a hallucinated tool call is the failure mode and the
  fall-back branch is load-bearing.
- **Kill switch path** — `.halt` detection → flatten → halt
  transition. Tiny LOC, infinite criticality.
- **Cancel / progress wiring in `crates/backtest/src/scenarios/`** —
  the Bug #63 surface. Every scenario must thread `cancel_rx` /
  `progress_tx`.

The lines that don't earn their coverage:

- **UI struct constructors / getters in `crates/ui/src/widgets/`** —
  generated boilerplate; covered transitively by snapshot tests.
- **`Display` impls on enums** — pure formatting, no branching.
- **`Debug` derives, `Clone` derives, frontmatter formatters.**
- **CLI argument parsing in `clap` derives.**

### Threshold strategy — tiered, not uniform

A workspace-wide 80% threshold is the wrong shape. It treats
`crates/ui` (where snapshot tests cover behavior more effectively
than line coverage measures) and `crates/strategy` (where every
branch IS the product) as the same animal.

My recommendation — **tiered per crate role**:

| Tier | Crates | Threshold | Rationale |
|---|---|---|---|
| **Hot** | `strategy`, `exec`, `audit`, `risk` | **90%** lines + **branches must hit both arms of Signal/SignalKind matches** | These ARE the product. The non-negotiable from CLAUDE.md (overlay e2e divergence) operates at this tier. |
| **Warm** | `core`, `reflection`, `llm`, `backtest`, `agent` | **80%** lines | Foundational, but some lines are pure infrastructure. |
| **Cool** | `ui`, `reports`, `cost`, `data` | **60%** lines + 100% on the `update`-equivalent state-transition function | UI's real coverage is snapshot tests, not line %. The state transition (`ui::state::update`) is the exception — see §4. |
| **Excluded** | `iced_tiny_skia` vendor, generated code | — | Out of scope. |

The branch-level rule on Hot crates ("both arms of `Signal/SignalKind`
matches must be hit") is the property-based threshold the operator's
question asked about. It's what *would have* caught the v3 no-op —
the `if (scale - 1.0).abs() < tol { passthrough } else { ... }`
arm where the else-branch had no behavioral consequence would
surface as a branch with execution coverage but no behavioural
coverage, which is the gap *mutation testing* answers — see below.

### Coverage vs mutation testing — which metric

**Coverage is the floor, mutation testing is the ceiling.** They
answer different questions:

- Coverage: "are my tests touching every line?"
- Mutation: "do my tests *detect* it when I change a line?"

The v3 no-op is the canonical case where coverage gives a green
light and mutation tests don't:

- `compute_scale` had 100% line coverage (8 unit tests).
- The `else` branch in `on_signal` was executed by every test
  where scale != 1.0 — so it had branch coverage too.
- A mutation that replaces `base_signals` (line 84 of the no-op
  bug site) with `base_signals.iter().map(|s| s.scale(2.0)).collect()`
  in the else-branch produces a *behaviorally different* return
  value. If no test fails under that mutation, the suite is not
  actually constraining the else-branch's semantics — which is
  exactly the bug.

For this project, with its "auditable + working" goals, **mutation
testing is a better fit than coverage as the headline metric**.
Coverage stays as the floor (cheap, every PR), mutation runs
periodically (weekly nightly?) over Hot-tier crates.

`cargo-mutants` is the obvious Rust tool ([mutants.rs](https://mutants.rs/)).
The architect-thread picks the tool; my strategic stance is
**adopt cargo-mutants for Hot-tier crates as a nightly periodic
gate, not blocking PR**. PR-blocking would be too slow (mutation
runs are minutes-to-hours); nightly with a punch-list of surviving
mutants reported to the operator is the right cadence. The UI
testability dev-note §2.12 already proposed this for `ui::state::update`
specifically; I'm extending the recommendation to the full Hot tier.

### Tooling stance

Between `cargo-llvm-cov` and `cargo-tarpaulin`: not my call (it's the
architect-thread's pick). My one strategic input: the tool MUST
produce **per-branch coverage**, not just per-line, because the v3
no-op's signature is at the branch level. Both tools support this;
the architect can pick on ergonomics + CI integration.

## §4 — Test pyramid shape — is the current shape right?

Verified rough shape (`grep '#\[test\]' crates/*/src/* crates/*/tests/*
| wc -l` = 783; 192 source files with `#[cfg(test)]`; 214 test files
under `*/tests/`; 49 anchored backtest reports; ~141 anchored reports
total under `spec/*/reports/`; 17 LLM-mocked tests via wiremock;
zero `_end_to_end.rs` files outside the lone `vol_targeting_overlay_end_to_end.rs`;
zero mutation tests in CI; zero VLM judges shipped).

That's roughly:

```
~ 700 unit tests (per-fn, mostly math, builder, parsing)
~ 200 integration tests in crates/*/tests/ (most are update-driven UI)
   49 anchored backtest reports
    1 cockpit-smoke gate (fixtures only)
   17 LLM mock/wiremock tests
   ~10 proptest invariant tests
    1 e2e overlay-divergence test
    0 mutation tests
    0 live-binary smokes
    0 VLM judges
```

### Is that the right shape?

For a classical Rails/Django app — the 70/20/10 unit/integration/e2e
shape is fine. For this project, given the product goals, **no**.
The shape inverts what the operator's reframed goal asks for.

The classical pyramid optimises for *speed of feedback at the unit
layer* and accepts that integration / e2e are slower + flakier and
should be minimized. That's correct in a context where the unit
layer's behavior composes predictably into the integration layer's
behavior.

In this project, the **composition is exactly where the bugs live**:

- v3 vol-target no-op — composition of `compute_scale` (unit-tested)
  with `on_signal` (untested at the composition layer).
- Bug #63 progress bar — composition of `runner.rs` (unit-tested
  via `lab_run_*.rs`) with cross-sectional scenarios (untested at
  the runner-composition layer).
- The chart-canvas-overhaul incident (ref:
  [`ui-testability-deep-dive-2026-05-15.md`](ui-testability-deep-dive-2026-05-15.md))
  — composition of `Canvas::Program::State` (untestable at the
  unit layer) with `Cockpit::chart_tooltip` (snapshot-tested in
  isolation).

A test pyramid where the composition layer is thin is structurally
incapable of catching these. **The right shape for this project is
diamond-ish, not pyramid**:

```
[narrow: live-binary smoke + VLM judge]
[wide:   composition / e2e / integration]
[wide:   unit + property + mutation]
```

Or in numbers, my POV for the next 6 months:

| Layer | Current | Target | Delta |
|---|---|---|---|
| Unit + property | ~700 | ~700 | stable |
| Integration (incl. `_end_to_end.rs`) | ~200 | ~300 | **+100** |
| Anchored backtest reports | 49 | 55 | +6 (one per shipping strategy) |
| Mutation tests (per Hot crate, nightly) | 0 | 4 | **+4** (strategy, exec, audit, risk) |
| Live-binary smoke | 0 | 1-3 | **+3** (cockpit_live Lab Run, kill switch, reflection retrieval) |
| VLM judges (shadow mode) | 0 | 1 | +1 ([per existing dev-note Q-VLM lock](ui-testability-deep-dive-2026-05-15.md#6-open-questions-for-the-operator)) |
| Hygiene tests (defensive grep) | 2 | 5 | **+3** (overlay e2e, no-op detector, etc.) |

The widening I'm proposing is at the **integration + mutation +
live-binary layer**. That's where the operator-readable failure modes
live, and that's where the project's product goals are most
under-served.

### The 70/20/10 doesn't fit auditable + reflective

Specifically: an audit ledger's correctness is a *property*, not a
behaviour, and properties are tested by property tests + mutation
tests, not by unit-tests-on-individual-fns. A reflection-memory
loop's correctness is an *end-to-end behavior* (decision → outcome
→ card → retrieval → next decision), and behaviors are tested by
e2e tests, not by unit tests on `store::write`.

The classical pyramid was designed for stateless web request/response
shapes. This project is **a stateful agent with a memory**. Different
shape, different tests.

## §5 — Operator-decide questions

Five operator-decide Qs surfaced by this review. Each has an
analyst-recommended default the operator can accept en bloc or
challenge individually.

### Q1 — Adopt `cargo-mutants` as a nightly CI gate for Hot-tier crates?

**Context**: §3 above. Mutation testing is the metric that would
have caught the v3 no-op. PR-blocking is too slow; nightly with
operator-readable punch-list is the right cadence.

**Options**:
- (a) Adopt for Hot-tier (strategy, exec, audit, risk) as nightly.
  Surviving-mutant report goes to operator.
- (b) Adopt for `crates/ui/src/state.rs` only (per UI testability
  dev-note §2.12), defer Hot-tier expansion.
- (c) Opt-in per feature ("the next feature touching strategy/
  runs mutation in shadow mode").
- (d) Don't adopt; keep relying on coverage + e2e.

**Analyst default**: **(a)** — nightly cargo-mutants on Hot-tier with
operator-readable punch-list. Roughly 1-2 dev-days to wire up; ongoing
operator cost is one weekly look at the punch-list. This is the single
highest-leverage change in the whole review.

### Q2 — Ship the overlay-e2e hygiene gate?

**Context**: §2 above. The CLAUDE.md rule is documented, not enforced.
A hygiene test like
[`crates/reflection/tests/no_strategy_caller.rs`](../../crates/reflection/tests/no_strategy_caller.rs)
would close the gap mechanically.

**Options**:
- (a) Ship hygiene test today; fail CI immediately for the 3
  currently-uncovered overlays.
- (b) Ship hygiene test in `#[ignore]` mode for 1 week; retrofit
  the 3 e2e tests during the window.
- (c) Ship hygiene test scoped to *new* overlay files only
  (grandfather the existing 3).
- (d) Don't ship; rely on code review.

**Analyst default**: **(a)** — ship today, fail loud. The rule was
authored *because* the v3 no-op shipped; carrying a quiet `#[ignore]`
or grandfather perpetuates the exact failure mode the rule exists
to prevent. The operator's reframed goal is "operator learns by
building"; making the failure structural is the most direct
encoding of that. Cost is 3 e2e tests authored over the next 1-2
weeks as the forcing function bites.

### Q3 — Coverage threshold: tiered per crate role or workspace-wide?

**Context**: §3 above. A uniform 80% threshold misprices `crates/ui`
and undersells `crates/strategy`.

**Options**:
- (a) Tiered per crate role (Hot 90% + branch-level / Warm 80% /
  Cool 60% + 100% on state-transition fn).
- (b) Workspace-wide 80% (simple, common).
- (c) No threshold; coverage is informational only.
- (d) Property-based threshold ("strategy must cover both branches
  of every Signal::kind match"), no % at all.

**Analyst default**: **(a)** with **(d)** as an additional gate on
Hot-tier. Coverage % is the floor; the property-based gate on
strategy/exec/audit/risk is the *meaningful* check. The architect-
thread picks the tooling (cargo-llvm-cov or cargo-tarpaulin); both
support per-branch.

### Q4 — Live-binary smoke (a Lab Run through cockpit_live)?

**Context**: §1 P4 + §4 above. The cockpit_live binary has no
automated smoke. Bug #63 is the witness.

**Options**:
- (a) Author one live-binary smoke that drives cockpit_live through
  a Lab Run on a tiny fixtures dataset, asserts progress reaches 100%,
  Stop button works, fills land in audit. Runs in CI on every PR
  touching `crates/ui/src/lab/`.
- (b) Author one live-binary smoke; runs only on `main` post-merge
  (catches regressions; doesn't block PRs).
- (c) Author the smoke but mark it `#[ignore]` and run nightly.
- (d) Don't ship a live-binary smoke; rely on fixtures-mode + manual
  operator verification.

**Analyst default**: **(b)** — `main` post-merge. PR-blocking is too
slow for a live-binary cycle; nightly is too lax for the operator-
critical surface. Post-merge catches regressions before they hit
the operator without the slow-PR friction. Cost: ~3 dev-days for
the smoke + CI runner. The UI testability dev-note §3.1
(`ui-inspect-mcp`) is the long-term answer but is locked at
"defer to cycle 4."

### Q5 — Adopt the UI testability dev-note's `ui-vlm-judge` proposal?

**Context**: §1 P7 above + the [UI testability dev-note](ui-testability-deep-dive-2026-05-15.md)
Q-VLM already locked at "shadow mode only." This Q is whether to
*start* the shadow window, not whether to gate on it.

**Options**:
- (a) Start the 2-week shadow window for `ui-vlm-judge` now.
  Three locked claims: tooltip visible, no elements overlap >50%,
  contrast >= 4.5:1. Operator reviews disagreement log at the end.
- (b) Defer until `ui-gallery-bin` ships (VLM cost amortizes
  better over a gallery).
- (c) Don't adopt; the byte-diff + insta + consistency tests cover
  the UI failure modes adequately.

**Analyst default**: **(b)** — defer until ui-gallery-bin ships.
The VLM cost amortises across the gallery's ~50 cells, and the
gallery is the natural shadow-mode surface. This isn't a reversal
of the existing Q-VLM lock; it's a sequencing call. ETA: ui-gallery-bin
is queued in the UI testability dev-note's "Cycle 1" (weeks 2-3 of
the bootstrap timeline).

### Sixth Q I want surfaced (bonus)

### Q6 — Author a "test message includes failure-mode hint" convention?

**Context**: §1 P7 above. The asymmetry between bug-driven tests
(which teach) and green-field tests (which mechanically assert) is
the operator-learning gap.

**Options**:
- (a) Add a convention to `.claude/agents/tester.md` and
  `.claude/agents/developer.md`: every assertion in Hot-tier crates
  carries an `expect("...")` or `assert!(_, "...")` message that
  names the failure class.
- (b) Audit existing tests in Hot-tier crates and retrofit messages.
- (c) Skip; this is a soft norm, not a gate.

**Analyst default**: **(a)** as a forward convention + opportunistic
retrofit during normal test edits. No big-bang retrofit. Cost: zero
dev-days; pure habit change documented in the agent contracts.

## §6 — Closing POV

The workspace tests **a lot**. 783 test functions across 214 files
plus 141 anchored reports is not a thin suite. The grades above are
not "you don't have enough tests"; they are "the test layer doesn't
match the product layer where this project's value lives."

The three structural changes that matter most:

1. **Hygiene gate for overlay e2e tests** (§2) — closes the v3 no-op
   class mechanically. Half a day of work, perpetual return.
2. **Mutation testing on Hot tier** (§3 / Q1) — closes the
   "math-layer-green-but-behavior-broken" class. The v3 no-op
   would not have shipped under this gate.
3. **Live-binary smoke for cockpit_live Lab Run** (§4 / Q4) — closes
   the Bug #63 class. The operator's daily surface gets a daily
   gate.

Adopt those three and the project's testing approach moves from
"comprehensive at the wrong layer" to "comprehensive at the layers
the product goals demand." Everything else in this review is
incremental on top.

The classical pyramid is the wrong shape because this project is
not a classical app. It is an auditable, reflective, stateful agent
where composition IS the product. The shape that fits is wide at
the composition layer with mutation + live-binary smokes capping
the top — a diamond, not a pyramid.

## Cross-references

- [`spec/product.md`](../product.md) — product goals; Differentiator
  (2) + (4) are the moat tests must defend.
- [`spec/architecture/11-regression-gate.md`](../architecture/11-regression-gate.md)
  — the byte-SHA anchor gate (P5 — A−).
- [`docs/dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md`](v3-vol-overlay-noop-discovery-2026-05-22.md)
  — the canonical no-op precedent and the lessons that motivated
  the CLAUDE.md rule.
- [`docs/dev-notes/ui-testability-deep-dive-2026-05-15.md`](ui-testability-deep-dive-2026-05-15.md)
  — sister review on UI specifically; the L0-L7 layer model and
  Q-VLM lock referenced in §5 Q5.
- [`spec/bug-log.md`](../bug-log.md) — Bug #63 is the Lab progress-
  bar witness for P4.
- [`crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`](../../crates/strategy/tests/vol_targeting_overlay_end_to_end.rs)
  — pattern reference for §2.
- [`crates/reflection/tests/no_strategy_caller.rs`](../../crates/reflection/tests/no_strategy_caller.rs)
  — hygiene-grep template for §2.
- [`crates/ui/tests/consistency.rs`](../../crates/ui/tests/consistency.rs)
  — second hygiene-grep template (UI-side).
- Parallel architect thread:
  [`testing-framework-audit-2026-05-25`](testing-framework-audit-2026-05-25.md)
  — tooling + topology + CI shape live there.
- `CLAUDE.md ## Non-negotiables` — overlay-e2e-divergence rule that
  motivated §2.

## Changelog

- 2026-05-25 (analyst): initial review. Graded 7 product-test
  alignment properties (P1-P7), surfaced the overlay-e2e-coverage
  gap as the load-bearing enforcement issue (D grade), positioned
  mutation testing on Hot-tier as the highest-leverage single
  change, proposed a diamond-shaped pyramid rather than the
  classical 70/20/10 because composition is where this project's
  bugs live, and floated 6 operator-decide Qs with analyst defaults.
  The top-3 alignment grades by gap are P2 (D, overlay wire-
  completeness), P4 (D−, operator workflow integrity), P7 (C,
  operator-readable failure modes). The top-3 operator-decide Qs
  are Q1 (nightly cargo-mutants on Hot-tier, default a), Q2 (ship
  overlay-e2e hygiene gate today fail loud, default a), Q4
  (live-binary smoke on `main` post-merge, default b).
