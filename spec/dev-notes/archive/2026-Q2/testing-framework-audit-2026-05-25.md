---
slug: testing-framework-audit-2026-05-25
status: review
owner: architect
updated: 2026-05-25
---

# Testing framework audit (2026-05-25)

Read-only architecture audit of the workspace's test topology. Triggered
by the stuck-progress-bar live regression (Bug #63 R8 follow-up) that
the existing tests missed despite green-on-green: the math layer was
unit-tested, the anchored reports were byte-stable, the state updaters
were unit-tested, but the channel-recipe-state-widget seam was tested
nowhere. Same shape as the v3-volatility-forecaster no-op
([v3-vol-overlay-noop-discovery-2026-05-22.md](v3-vol-overlay-noop-discovery-2026-05-22.md)) —
five gate layers, each blind to the load-bearing property.

This note is **topology + tooling**. Strategic / product framing lives
in the parallel `testing-strategy-review-2026-05-25.md`. Both are
read-only; no code or spec changes here.

## §1 Inventory of test layers

### Aggregate counts (read from disk 2026-05-25)

| Metric | Count |
|---|---:|
| `#[test]` + `#[tokio::test]` function declarations across `crates/` | **2 055** |
| `#[tokio::test]` only | 372 |
| `#[cfg(test)]` module-bearing source files in `crates/*/src/` | 189 |
| Integration test files (`crates/*/tests/*.rs`) | **212** |
| Doctest `````rust` fence blocks | 130 (in only **2** source files — `crates/forecast/src/features.rs` and one peer; trivially under-utilised) |
| `proptest!` macro invocations | 21 (across 10 files) |
| `insta::assert_*` invocations | 18 files (~80 panel snapshots + ~20 widget-tree snapshots) |
| Bound iced_test usage | 17 files (1 crate — `ui`) |
| Files containing `byte_identical` / `determinism` assertions | 30 |
| `#[ignore]` annotations (quarantine) | **22** |
| Anchored backtest reports under `spec/*/reports/backtest-*.md` | 46 (against **34** anchors in `spec/anchors.toml`) |

`#[test]` counts per crate (lib + integration):

```
ui:           593     strategy:     306     audit:        171
reports:      171     forecast:     151     llm:          127
core:          98     data:          90     backtest:      77
agent:         73     features:      55     reflection:    50
cost:          14     risk:          10     exec:           9
replay-cache:   8     models:         0
```

**`models` has zero tests.** That's a leaf crate with little code so
plausibly benign, but it deserves a sweep. `risk` at 10 is also thin
for a crate the trading agent depends on; most of its work routes
through `core`'s tests but the unit story is sparse.

### 1.1 Unit tests (`#[cfg(test)] mod tests` inside `src/*.rs`)

- **Canonical example**: `crates/core/src/tests/order_tests.rs` (uses
  proptest for order-state invariants).
- **Coverage skew**: 189 source files carry inline tests; the heaviest
  inline-test crates are `forecast/src/*.rs` (training math, GARCH,
  TCN) and `features/src/*.rs` (every indicator gets a proptest in its
  own file: `sma.rs`, `rsi.rs`, `macd.rs`, `bbands.rs`, `ema.rs`,
  `cross_sectional.rs`). The `ui` crate's heaviest inline tests live
  in `crates/ui/src/widgets/*.rs` (16+ widgets each carrying their
  own `insta::assert_snapshot!` block).
- **Gap**: zero unit tests in `crates/models/`. `crates/risk/` ships
  10 — disproportionate for a safety-critical layer.

### 1.2 Integration tests (`crates/*/tests/*.rs` standalone files)

212 files across 13 crates. Distribution:

```
ui:        37    audit:    36    reports: 33
forecast:  21    llm:      19    strategy: 16
agent:     15    data:      9    backtest: 8
reflection:10    core:      5    cost:     2
exec:       1    others (no /tests dir): features, models, replay-cache, risk
```

- **Canonical example** (engine boundary): `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs:1` —
  exactly the shape mandated by CLAUDE.md's non-negotiable for every
  overlay / sizing modifier.
- **Canonical example** (UI integration): `crates/ui/tests/cockpit_live_kill_button_writes_audit.rs`
  drives `Message::*` through `update()` against an in-memory audit
  `Ledger` — pure state, no rendering.
- **Gap — entire crates with no integration tests**: `features`,
  `models`, `replay-cache`, `risk`. Of these, `risk` is the
  glaring one — every risk-gate decision is unit-tested but the
  crate has no `tests/` directory. `exec` ships only 1 integration
  test (`crates/exec/tests/`).

### 1.3 Doctests (`cargo test --workspace --doc`)

130 ````rust` fence blocks but only in 2 source files
(`crates/forecast/src/features.rs` and `crates/forecast/src/patchtst.rs`).
The workspace is essentially **doctest-empty** by Rust ecosystem
standards. `precheck.sh` exists specifically to catch the
`[package] name = "core"` shadowing bug that would explode doctests,
but the surface area being protected is tiny.

- **Action implied**: every public API in a library crate should ship
  with at least one `/// ```` doctest, both for the documentation
  payload and as a smoke test that the public API compiles. This is a
  ~4-week organic backfill, not a sprint.

### 1.4 Property tests (`proptest`)

- **In `Cargo.toml`** of: `core`, `features`, `forecast`, `reflection`,
  `reports`, `risk`, `strategy`, `ui`. **Not in**: `audit`, `agent`,
  `backtest`, `cost`, `data`, `exec`, `llm`, `models`, `replay-cache`.
- **Heaviest user**: `crates/features/src/*.rs` (every indicator) +
  `crates/ui/tests/layout_invariants.rs` (the M1-C zero-dim Node
  invariant — 6 widgets fuzzed at 256 cases each with ChaCha-seeded
  determinism per `ProptestConfig::with_rng_algorithm(RngAlgorithm::ChaCha)`).
- **No `quickcheck`** anywhere — proptest is the chosen tool.
- **Gap**: the property-test surface is concentrated in features +
  layout invariants. Stateful property testing
  (`proptest-state-machine` over `ui::state::update`) is **not yet
  adopted** despite being called out in
  [ui-testability-deep-dive-2026-05-15.md § 2.3 + § 3.4](ui-testability-deep-dive-2026-05-15.md).
  `crates/strategy/` has 306 tests but zero property tests against
  the strategy trait — the natural surface (e.g. "a strategy that
  short-circuits on no signal must never increment fill counters")
  is unguarded.

### 1.5 Snapshot tests (`insta`)

- **In `Cargo.toml`** of only **2 crates**: `audit`, `ui`. Everything
  else avoids `insta` even where it would help.
- **The `ui` crate's 80 panel snapshots** at
  `crates/ui/tests/panel_snapshots.rs` are **text summaries** of state
  (per the file's docblock at lines 1-19) — they never exercise the
  iced widget tree. This is the same documented weakness from the
  Brief A F1-cockpit-render-regression incident and is correctly
  noted in `cockpit-smoke/SKILL.md:10`. 80 tests of state-to-string,
  zero of state-to-pixels at this layer.
- **86 `.snap` files** under `crates/ui/tests/snapshots/`.
- **Widget-tree assertions are NEW** as of 2026-05-25: the triage dev's
  `crates/ui/tests/progress_bar_widget_label.rs` introduced the
  `iced::advanced::widget::Tree::new(element.as_widget()).children.len()`
  discriminant pattern. This is the first widget-tree-shape (not pixel,
  not text) assertion in the repo. **It is one widget deep.** Every
  other widget under `crates/ui/src/widgets/` does NOT have this
  treatment yet.

### 1.6 Anchored backtest tests (body-SHA-256 immutable contract)

- 34 anchors in [`spec/anchors.toml`](../anchors.toml) (the prose calls
  this "the 9-anchor regression gate" — that's stale; counted today
  it is 34. The original `9` set evolved as v0.5, v1, v3 strategies
  shipped and locked their own anchors).
- 46 anchored report files on disk (some scenarios have duplicates
  staged for `prune_backtest_duplicates.sh`; see
  [verify-anchors/SKILL.md § Post-PASS bookkeeping](../../.claude/skills/verify-anchors/SKILL.md)).
- **Hard gate**: `scripts/verify_anchors.sh` — invoked by the tester
  per AGENT.md § 3 "Anchor gate" rule. Exit 0 required before
  `VERDICT → PASS`.
- **Documented blindspot from 2026-05-22**: byte-identity is the
  *signature* of a no-op overlay; the gate cannot interpret what
  it witnesses. See [v3-vol-overlay-noop-discovery-2026-05-22.md § "Why
  the gates didn't catch it" — row 3](v3-vol-overlay-noop-discovery-2026-05-22.md).

### 1.7 Widget render tests

- **Before 2026-05-25**: zero. Widgets had `view()`-doesn't-panic
  tests (e.g. `progress_bar::view(None, None, mode)` was driven only
  to assert the call returned without unwinding) but no structural
  assertion about the rendered tree.
- **After 2026-05-25 triage**: the dev's
  `crates/ui/tests/progress_bar_widget_label.rs` is the new template:
  8 tests asserting `Tree::new(el.as_widget()).children.len()` equals
  2 (label-present) or 0 (label-absent). It's the cheapest possible
  structural assertion — no rendering, no font, no fixture state.
  It costs **~3 lines of test per assertion** and runs in <1 ms.
- **Pattern not yet propagated**. The directory `crates/ui/src/widgets/`
  contains ~30+ widget files; only `progress_bar.rs` has the
  widget-tree treatment. The other 29 are vulnerable to the same
  shape of bug.

### 1.8 End-to-end subscription tests (channel → recipe → state → render)

- **Before 2026-05-25**: zero true end-to-end tests covering the
  `backtest::progress_pair() → LabProgressRecipe::stream() →
  ui::state::update(Message::LabRunProgress) → widget render`
  pipeline. The three segments were unit-tested in isolation; the
  wiring was not.
- **After 2026-05-25 triage**: `crates/ui/tests/lab_progress_recipe_stream.rs`
  drives `stream_impl(Some(rx))` end-to-end and validates the
  happy-path (`LabRunProgress → LabRunProgressDone`) AND the smoking-gun
  path (`stream_impl(None)` yields nothing) — 8 tests, ~240 LOC.
  This is **the test class that would have caught Bug #63** had it
  existed pre-incident. It is excellent forensic work; it is also
  one channel.
- **Other subscription tests in the repo**:
  `crates/ui/tests/live_subscription.rs`,
  `live_subscription_full_bus.rs`,
  `risk_telemetry_subscription.rs`. These test live-tick subscriptions
  for cockpit operation; they are NOT end-to-end through to widget
  render — they assert state mutation after message delivery.

### 1.9 Gallery snapshot tests (`crates/ui/src/gallery/`)

- **Crate role**: `crates/ui/src/gallery/{cell.rs, mod.rs, routes.rs}` +
  the `ui-gallery` binary (one of 18 workspace bins, see
  `crates/ui/Cargo.toml`). Each "cell" is a single widget × state ×
  viewport combination rendered on a long scrolling page; one
  `iced_test::screenshot()` captures 50+ baselines at once.
- **Status — BLOCKED on iced 0.14 `Table` interaction**.
  `crates/ui/tests/gallery_snapshots.rs:1-30` documents the
  `GALLERY_CELLS[7]` (`strategies :: ready_v1`) panic at
  `iced_tiny_skia::engine.rs:686` "Build quad rectangle". V5+ blocked,
  partial ship V1-V4 only. The three slot tests are `#[ignore]`d.
  Tracked as `ui-gallery-table-cell` follow-up.
- **Implication**: the highest-ROI agent artifact called out in the
  2026-05-15 deep-dive (§ 2.13 + § 3.3) **is half-shipped and stalled**.
  The bisect test at `crates/ui/tests/gallery_bisect.rs` exists to
  localize the panic on each iced rev.

### 1.10 Cockpit-smoke gate (`.claude/skills/cockpit-smoke/`)

- **What it is**: orchestrator-only `cargo run -p ui --bin cockpit
  --features fixtures` for 7 s, grep stderr for `panicked at` lines,
  exit 0 if clean.
- **Coverage**: catches first-frame `fill_quad` / `unreachable!()` /
  zero-dim Quad panics. Mandatory orchestrator gate per AGENT.md § 6
  process discipline rule, between evaluator PASS and presenter
  assembly.
- **Explicit non-coverage** (per skill docs § "False-negative
  envelope"): silent visual regressions (palette drift, layout shift
  that doesn't panic), input interaction panics (the 7 s window is
  frame-0 only), multi-frame regressions.
- **Fixtures-mode only**. Has not been expanded to a "live-data
  fixtures" mode that would exercise the subscription pipe.

### 1.11 Determinism / byte-identity tests

30 files mention determinism or byte-identity. Major exemplars:

| Test file | Asserts |
|---|---|
| `crates/forecast/tests/tcn_byte_identity.rs` | TCN inference is bit-identical across two runs at the same seed (currently `#[ignore]`d on Metal — see `metal_cpu_drift.rs`) |
| `crates/forecast/tests/garch_fit_determinism.rs` | GARCH MLE convergence is byte-identical across two runs |
| `crates/forecast/tests/patchtst_byte_identity.rs` | PatchTST inference byte-identical |
| `crates/forecast/tests/forward_determinism_patchtst.rs` | PatchTST forward pass byte-identical |
| `crates/forecast/tests/metal_cpu_drift.rs` | Drift envelope between Metal and CPU inference paths (the falsifier for "Metal is byte-identical") |
| `crates/reports/tests/determinism.rs` | Two runs of `lib::generate` 10 s apart produce identical body SHA, differing front-matter |
| `crates/backtest/tests/determinism.rs` | Backtest binary determinism (the regression-gate complement) |
| `crates/backtest/tests/multi_pair_determinism.rs`, `multi_symbol_determinism.rs` | Cross-sectional path determinism — added 2026-05-23 |

**Strong layer.** The determinism tests are well-distributed across
the forecast pipeline and the reports rendering layer; backtest has
both single and multi-pair coverage. This is the layer the v3-vol-overlay
incident broke through (byte-identity passed exactly *because* the
overlay was a no-op).

### 1.12 Cross-crate contract / hygiene tests

- `crates/reflection/tests/no_strategy_caller.rs` — the
  canonical "static grep as a test" pattern. Walks
  `crates/strategy/src/` and fails if any file references
  `reflection::retrieve_top_k` (etc).
- `crates/ui/tests/consistency.rs` — same shape, walks
  `crates/ui/src/widgets/` and fails if any file contains an inline
  user-visible string literal or a `#[0-9a-fA-F]{6}` hex code (i.e.
  enforces theme-routing).
- `crates/reports/tests/body_no_volatile_metadata.rs` — checks the
  body-vs-front-matter discipline (the HF-1 / T715 regression class).
- `crates/data/tests/funding_poller_integration.rs` — cited in the
  grep but is more of an integration than hygiene test.

Shell-level hygiene gates that complement these:

- `scripts/check_no_clocks_in_ui_tests.sh` — forbids
  `SystemTime::now()` / `Instant::now()` in the UI rendering paths
  reachable from `iced_test::screenshot`. Allow-list via `// CLOCK-OK:`
  inline marker.
- `scripts/check_no_secrets_in_llm_artifacts.sh` — V9 grep gate for
  `sk-` / `sk-ant-` / `bearer` patterns in LLM artifacts.
- `scripts/precheck.sh` — `[package] name = ` stdlib-shadowing
  gate (`core`, `std`, `alloc`, `test`, `proc_macro`).
- `scripts/spec_lint.py` — `dead-link`, `missing-frontmatter`,
  `orphan-feature`, `bad-anchor`, `unreferenced-anchor`,
  `shipped-no-tests`, `trace-broken-path`, `adr-not-registered` gates.
- `scripts/verify_anchors.sh` — the 34-anchor body-SHA gate.

### 1.13 CLI / binary smoke tests

- **18 workspace binaries** (`grep '\[\[bin\]\]' crates/*/Cargo.toml`):
  `trading`, `backtest`, `threshold_sweep`, `fetch_binance_klines`,
  `fetch_yahoo_klines`, `train_garch`, `vol_verdict`, `train_tcn`,
  `forecast_distribution`, `recalibrate_sigma_train`,
  `train_patchtst`, `sharpe_comparison`, `llm_verdict`, `report`,
  `cockpit`, `ui-gallery`, `cockpit_live`, `viewer`.
- **`assert_cmd` crate is absent from every `Cargo.toml`**. There is
  no canonical "run the binary, assert stdout/exit code" pattern.
  Existing CLI smoke tests are scattered:
  - `crates/forecast/tests/train_tcn_golden_cli.rs`
  - `crates/forecast/tests/train_tcn_dry_run.rs`
  - `crates/forecast/tests/train_tcn_audit_emits.rs`
  - `crates/forecast/tests/forecast_distribution_bin_readonly.rs`
  - `crates/forecast/tests/recalibrate_sigma_train_readonly.rs`
  - `crates/backtest/tests/backtest_sharpe_emit_equity_bin.rs`
  - `crates/backtest/tests/threshold_sweep_readonly.rs`
  These call `std::process::Command::new("cargo")` directly. No
  shared helper. **Gap**: 8 of the 18 binaries have no smoke test
  whatsoever (`trading`, `fetch_binance_klines`, `vol_verdict`,
  `train_patchtst`, `sharpe_comparison`, `llm_verdict`, `report`,
  `viewer`).

## §2 Code coverage state

**Verdict**: **No coverage tool is configured anywhere in the workspace.**

Searches performed:

```
grep -r "llvm-cov\|tarpaulin\|grcov" Cargo.toml crates/*/Cargo.toml \
  scripts/ .claude/ spec/architecture.md
# → 0 matches.
```

There is no `.github/workflows/` directory. There is no `Makefile`.
There is no `rust-toolchain.toml`. **There is no CI server at all** —
the test orchestration is agent-driven (tester sub-agent invoked
by the orchestrator). This is consistent with the
[MEMORY.md feedback_no_worktrees] policy: "work directly on `main`
in the main repo".

No coverage threshold is enforced anywhere. `spec/architecture.md`
does not mention coverage.

### Recommendation

**Adopt `cargo-llvm-cov`** as the canonical coverage tool. Justification:

- Most mature today (2025/2026). `cargo-tarpaulin` has historically had
  issues with proc-macro / async correctness on macOS+Apple-Silicon;
  `cargo-llvm-cov` rides the LLVM source-based coverage directly and
  is documented to work cleanly with `cargo test --workspace`.
- Pure-Rust install via `cargo install cargo-llvm-cov`. No C
  dependencies (good for the single-binary architecture).
- Native HTML report generator + Cobertura/JSON export.
- Edition-2024 compatible.
- Anthropic's internal tooling and the Rust community both default
  here today.

**Integration shape (no GitHub Actions; agent-driven):**

1. Add a new skill `.claude/skills/rust-coverage/SKILL.md` that runs
   `cargo llvm-cov --workspace --all-features --lcov --output-path
   /tmp/lcov.info` and emits a per-crate coverage table plus the top-10
   uncovered functions.
2. Wire the skill into the tester fan-out (per AGENT.md § 5
   "Tester fan-out") as a **non-gating warning** for two weeks. Log
   the per-crate coverage at every tester run. The data feeds an
   initial threshold-setting decision.
3. After two weeks, set per-crate floors:
   - `core`, `audit`, `risk`, `exec`, `backtest`, `strategy`, `reports` →
     85 % line coverage floor (these are the load-bearing strategy /
     accounting / safety layers).
   - `ui` → 60 % (rendering layers are deliberately under-tested by
     pixels; the gallery/widget-tree work fills the rest).
   - `data`, `llm`, `agent`, `forecast`, `features`, `reflection`,
     `cost`, `models`, `replay-cache` → 70 % default.
4. Promote to gating after first stable two-week window. A drop
   below floor blocks `VERDICT → PASS`.

**Effort estimate**: ~3 hours to author the skill + tester wiring; ~1
hour per crate to interpret the first report (so ~15 hours of
analyst/architect review across the 15 crates). The tooling install
is `cargo install cargo-llvm-cov` once. **Total: ~1 dev-day to
operational, ~1 week to gated.**

## §3 CI test orchestration

### Topology

There is **no GitHub Actions / GitLab CI / Jenkins / Buildkite
runner**. The workspace's continuous-integration system IS the
multi-agent orchestrator described in
[AGENT.md § 5 Tester fan-out](../../AGENT.md). The "CI gates" are:

| Stage | Tool | Where invoked |
|---|---|---|
| Format | `cargo fmt --all -- --check` | `rust-validate` skill |
| Lints | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | `rust-validate` skill |
| Dep audit | `cargo audit` | `rust-validate` skill |
| Policy | `cargo deny check` | `rust-validate` skill (when `deny.toml` exists — it does) |
| Docs build | `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | `rust-validate` skill |
| Stdlib-shadow | `scripts/precheck.sh` | pre-flight (orchestrator + dev) |
| Spec hygiene | `uv run scripts/spec_lint.py` | mandatory tester pre-tick + presenter pre-tick |
| Lib tests | `cargo test --workspace --lib -- --nocapture` | `rust-test` skill step 2 |
| All targets | `cargo test --workspace --all-targets -- --nocapture` | `rust-test` skill step 2 |
| Doctests | `cargo test --workspace --doc` | `rust-test` skill step 2 |
| Proptest expanded | `PROPTEST_CASES=1024 cargo test --workspace property_` | `rust-test` skill step 3 (conditional on proptest presence) |
| Backtests | `cargo run --release --bin backtest -- --scenario <name>` | `backtest` skill |
| Anchor gate | `scripts/verify_anchors.sh` | mandatory after touching strategy/audit/exec/backtest/report code |
| UI clock gate | `scripts/check_no_clocks_in_ui_tests.sh` | `rust-validate` extension |
| Secrets gate | `scripts/check_no_secrets_in_llm_artifacts.sh` | tester pre-`VERDICT` |
| Cockpit smoke | `cargo build -p ui --bin cockpit --features fixtures && 7s window + grep` | **orchestrator-only** (capability boundary), every UI brief evaluator PASS |
| Benches | `cargo bench --workspace -- --save-baseline current --baseline main` | `rust-bench` skill, criterion outputs in `target/criterion/` |

### Plug-in points for `rust-test`

The `rust-test` skill ([`.claude/skills/rust-test/SKILL.md`](../../.claude/skills/rust-test/SKILL.md))
is **tester-agent-only** today. Per `AGENT.md § 5`, the tester runs
`rust-validate`, `cargo test`, `rust-bench`, `backtest`, and
`spec-lint` as parallel sub-agents and merges their outputs. The
orchestrator does NOT run `rust-test` itself — only the
`cockpit-smoke` gate is orchestrator-direct (capability boundary —
sub-agents may not launch the cockpit binary).

There is **no pre-commit hook** in the repo. Hygiene is enforced at
tester-run time, not at `git commit` time.

### Wall-clock bandwidth

From the lab-yahoo-realdata 2026-05-24 final test report
(`spec/lab-yahoo-realdata/reports/test-final-2026-05-24.md`):

- `cargo test --workspace --lib`: ≥ 878 passed, ~6 s aggregate per-crate
  duration table. End-to-end including compile is ~5 minutes warm.
- `cargo test --workspace --all-targets` (the full sweep): **~25 minutes
  cold per the R1 dev's recent reading** (cited in the operator's
  prompt; verified plausible against the test counts — 2 055 functions
  × proptest expansions × determinism repeat-twice tests + iced_test
  screenshot generation on tiny-skia + backtest invocations).
- Criterion benches: not included in the test sweep; invoked
  on-demand by `rust-bench`.

### Quarantine

**22 `#[ignore]` annotations** in 18 files. Notable:

| Quarantined test | Why |
|---|---|
| `crates/ui/tests/gallery_snapshots.rs` (3 slot tests) | iced 0.14 Table-cell render panic; tracked as `ui-gallery-table-cell` |
| `crates/ui/tests/render_snapshots.rs` | needs operator-side baseline approval; sandboxed sub-agents can't approve |
| `crates/ui/tests/lab_yahoo_anchor.rs`, `lab_yahoo_dispatch.rs` | block on operator populating Yahoo cache (Bug #61 partial) |
| `crates/ui/tests/lab_run_real_engine.rs` | live-data path; runs only with Yahoo fixtures populated |
| `crates/ui/tests/gallery_bisect.rs` | diagnostic tool, not a regression assertion |
| `crates/data/tests/binance_ws_integration.rs`, `binance_tick.rs`, `yahoo_revision_verify.rs` | live-network integration tests |
| `crates/forecast/tests/tcn_byte_identity.rs` | Metal-vs-CPU drift envelope (the falsifier itself) |
| `crates/forecast/tests/patchtst_overlay_neutrality.rs` | needs trained-model checkpoint not in git |
| `crates/forecast/tests/sigma_train_not_in_safetensors_patchtst.rs` | known training artifact gap |
| `crates/strategy/tests/llm_forecaster_neutrality.rs` | wiremock LLM neutrality envelope, runs on-demand |
| `crates/backtest/tests/determinism.rs` | live data path |

**The two tests the operator's prompt flagged**:

- `consistency::no_inline_user_visible_strings_in_widgets` —
  pre-existing regression in `crates/ui/tests/consistency.rs`. **Not
  formally quarantined** (no `#[ignore]`). The lab-yahoo-realdata
  test report records it as a known-failure tolerated through the
  ship gate (cited at `spec/lab-yahoo-realdata/reports/test-final-2026-05-24.md`).
  This is the **most concerning quarantine pattern in the repo**:
  the test runs, fails, and the failure is *normalized*. The
  hygiene gate is functionally retired without a marker.
- `cockpit_live_kill_button_writes_audit` — flaky per the R1 dev's
  handoff. Not `#[ignore]`d on disk. Same pattern.

**Quarantine mechanism gap**: there is no central `quarantine.toml`
or equivalent registry. A test is either `#[ignore]` (well-marked) or
silently tolerated (the consistency case). Bringing the latter into
the former is one of the §5 recommendations.

## §4 Gap analysis (ranked)

### G1 — Channel-recipe-state-widget seam has no canonical test class. SEVERITY: HIGH.

**Representative bug**: Bug #63 R8 — `LabProgressRecipe::stream_impl(None)`
silently yields nothing → UI bar frozen at 30 %. Math layer, state
layer, and widget layer all green. Channel pipeline untested.

The new `lab_progress_recipe_stream.rs` closes Lab progress
specifically; no analogous coverage exists for:

- `crates/ui/src/lab/runner.rs` Yahoo fetch streaming
- `crates/agent/` event stream → cockpit subscription
- `crates/audit/` LLM-budget-event delivery
- Any other tokio mpsc → iced `Recipe` pattern in the codebase

The general pattern: **find every `Recipe` impl + every
`subscription::*` function and confirm at least one end-to-end test
exercises it**. The grep surface is small (probably ≤ 10 sites);
authoring a canonical end-to-end test class would close the entire
gap in ~2 dev-days.

### G2 — CLAUDE.md non-negotiable "overlay must ship with baseline-equity-divergence end-to-end test from day 1" is **not mechanically enforced**. SEVERITY: HIGH.

**Representative bug**: v3-vol-overlay no-op (May 22). The CLAUDE.md
rule was added as a *response* to this incident, but nothing
**prevents** the next overlay from skipping the test. The reference
implementation `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`
is the template; there is no test or lint or `spec-lint` rule that
asserts "every type implementing `Strategy` with `overlay` in its
name has a matching `*_end_to_end.rs` integration test".

Concrete enforcement options:

- **A. `spec-lint` rule extension**. New category `overlay-missing-e2e`:
  walk `crates/strategy/src/*overlay*.rs`, assert a sibling
  `crates/strategy/tests/*overlay*_end_to_end.rs` exists. ~30 LOC
  Python. Doesn't catch the *content* of the test (only existence).
- **B. Compile-time enforcement via trait**. Introduce
  `trait OverlayE2eTest` with a `fn proof_of_divergence() -> impl
  Future<Output = f64>` method that returns the bps divergence; the
  framework runs it and asserts ≥ 1 bp. Heavyweight but compiler-enforced.
- **C. Architect ADR review checklist**. Every new overlay ADR must
  cite the test path. Human-process, not mechanical.

Architect recommendation: A + C (mechanical existence + ADR cite).
B is over-engineering for a < 10-overlay surface.

### G3 — `cockpit-smoke` is fixtures-only; the live subscription path is unmoderated. SEVERITY: MEDIUM.

The gate catches frame-0 panics in fixtures mode. The Bug #63 pattern
(channel-recipe wiring) does NOT panic — it silently freezes. A
fixtures-mode 7s window cannot exercise the live tokio runtime
because there is no live data feed in fixtures mode.

The gap deserves a paired smoke gate: a `cockpit-smoke-live` that
boots `cockpit_live` against synthetic-but-real-runtime data (e.g.
the existing `live_subscription_full_bus.rs` fixtures wired into the
cockpit live binary) for a 15-30 s window and asserts the progress
bar / spinner / data feeds **actually advance**. A second skill,
orchestrator-only.

### G4 — Widget render tests cover one widget. SEVERITY: MEDIUM.

`progress_bar_widget_label.rs` is the template. The other ~29 widgets
under `crates/ui/src/widgets/` have no `Tree::new(el.as_widget())`
assertions. Easy follow-up: a one-sprint backfill that gets every
widget to "label-present and label-absent variants differ in child
count" (or the closest analog per widget). ~3 dev-days for the
sweep.

### G5 — Cross-sectional scenario coverage was incomplete pre-Bug #63. SEVERITY: MEDIUM (now mitigated).

Per Bug #63 commit `982830f`: cross-sectional scenarios (`momentum`,
`pairs`, `tcn_overlay`) never took `cancel_rx` / `progress_tx`. Stop
button silent; progress bar frozen. The triage fix added
`multi_pair_determinism.rs`, `multi_symbol_determinism.rs`, and the
cross-sectional cancel coverage — this gap is **now closed but the
discovery pattern is generalizable**.

The general statement: **every binary-level capability (cancel,
progress, retry, timeout) should have a per-scenario test matrix
rather than a single sma_crossover test**. The `spec-lint` rule
"any scenario name without per-capability coverage" is a candidate.

### G6 — `models`, `risk`, `replay-cache`, `features` lack an integration `tests/` directory entirely. SEVERITY: MEDIUM.

- `models` (0 #[test]) — possibly benign if the crate is thin; needs
  a sweep to confirm.
- `risk` (10 #[test], all unit) — safety-critical. The risk-veto
  audit-trail behavior is only tested through `audit::tests::risk_veto_overridden.rs`
  (an `audit`-side cross-cutting test). The `risk` crate itself
  has no boundary test that "given a portfolio at threshold N+1,
  the engine emits a veto Message".
- `replay-cache` (8 #[test]) — low-traffic but should at least have
  one integration test of cache hit/miss / TTL semantics.
- `features` (55 #[test], all proptest unit) — the indicator math is
  thoroughly covered; the streaming-feature integration (multi-bar
  rolling windows) is not tested at the boundary.

### G7 — Doctest payload is functionally empty. SEVERITY: LOW.

130 fence blocks in 2 files. A library project of this size should
have hundreds of doctests across public APIs. This is a "cultural"
gap rather than an incident-driving one, but it costs the agent
real value (no doctests means the LLM has no machine-checkable
examples of how to call the API).

### G8 — Quarantine without an `#[ignore]` marker (the `consistency::no_inline_user_visible_strings_in_widgets` pattern). SEVERITY: HIGH (silent gate erosion).

The test runs every sweep, fails every sweep, and is tolerated by
the tester report. This is gate erosion in slow motion. Either
re-instate the gate (and fix the underlying violations) or
`#[ignore]` it with a `# Reason:` annotation referencing a
follow-up brief. Choose explicitly; do not let the green tester
report fool the operator into believing the gate is intact.

### G9 — 8 of 18 workspace binaries have no smoke test. SEVERITY: LOW-MEDIUM.

`trading`, `fetch_binance_klines`, `vol_verdict`, `train_patchtst`,
`sharpe_comparison`, `llm_verdict`, `report`, `viewer` — none have
a `crates/*/tests/*_bin.rs` exit-code + stdout test. A `--help`
smoke at minimum (per binary, ~10 lines of `Command::new("cargo")`
in a shared `tests/common.rs` helper using `assert_cmd`) is one
sprint's worth of mechanical work.

### G10 — Statefull property tests over `ui::state::update` not adopted despite being called out 10 days ago. SEVERITY: MEDIUM.

[ui-testability-deep-dive-2026-05-15.md § 2.10 + § 3.4](ui-testability-deep-dive-2026-05-15.md)
quantified the gap: ~40 of ~60 `Message` variants are untested
through the update function. The proposal (`proptest-state-machine`
over `ui::state::update`, 5 invariants) was scoped at 5 dev-days.
Ten days later, no progress. The gap remains.

## §5 Recommendations (ranked, with effort)

### R1 — Promote "channel-recipe-state-widget end-to-end" to a first-class layer with a `subscription-pipe` skill. [HIGHEST ROI]

**Effort**: 3 dev-days.

**Action**: catalog every `iced::subscription::Recipe` impl + every
`tokio::sync::mpsc` channel reaching iced. For each, author a
sibling test using the `lab_progress_recipe_stream.rs` template:

- happy path: sender pushes N messages → recipe yields N + 1 (done)
- silent path: `stream_impl(None)` or equivalent → empty yield
  documented and asserted
- backpressure: bounded channel full → graceful drop or block per
  the channel's contract

Pair with a `spec-lint` rule `subscription-missing-e2e` that fails
when a new Recipe impl lands without a matching test file.

**Why first**: Bug #63 cost a live regression. The template is
already authored. The next bug of the same shape is queued in some
other channel; this closes the entire class.

### R2 — Adopt `cargo-llvm-cov` in non-gating warning mode, 2-week soak, then per-crate floors. [HIGH ROI]

**Effort**: 1 dev-day for skill + tester wiring; 2 weeks of data
collection; 1 dev-day to set floors.

**Action**: per § 2. Defaults:

- Tool: `cargo-llvm-cov` (cargo install, no C deps, edition-2024
  clean).
- New skill: `.claude/skills/rust-coverage/SKILL.md` driven by
  `cargo llvm-cov --workspace --lcov --output-path /tmp/lcov.info`
  with a per-crate aggregation step.
- Wired into tester fan-out (per AGENT.md § 5) as a sixth parallel
  sub-agent.
- Two-week shadow → set floors → gate on regression below floor.

**Why second**: the audit surfaces several gaps (G6, G7, G9) that
coverage data quantifies mechanically. Currently every claim about
"crate X is undertested" is from gut. Coverage data lets the
operator decide where to spend.

### R3 — Adopt `cargo-mutants` once, scoped to the highest-risk surfaces. [MEDIUM-HIGH ROI]

**Effort**: 2 dev-days for the first scoped run + triage.

**Action**: re-affirms the `ui-mutants-pass` recommendation from
[ui-testability-deep-dive-2026-05-15.md § 3.7](ui-testability-deep-dive-2026-05-15.md).
Scope to the four files where math-vs-wiring decoupling is
load-bearing:

```
cargo mutants --package strategy --file crates/strategy/src/vol_targeting_overlay.rs
cargo mutants --package strategy --file crates/strategy/src/tcn_overlay_momentum.rs
cargo mutants --package risk     --file crates/risk/src/portfolio.rs
cargo mutants --package ui       --file crates/ui/src/state.rs
```

Each run produces a triage report. Surviving mutants in the math
arm are tolerable (the math is byte-identity-tested). Surviving
mutants in the wiring or composition arm are exactly the G2 /
v3-vol-overlay class. Operator triages the top 10 per file.

**Why not first**: cargo-mutants is slow (10s of minutes per file,
~hours per run). It produces a 1-time signal, not a per-sweep gate.
Best run quarterly.

**Why not VLM judge yet**: per the deep-dive's § 2.9 + § 3.2 risk
analysis, VLM judges flake on prompt drift, model updates, and
synonym ambiguity. The "shadow mode for 2 weeks then maybe gate"
protocol is correct but premature: we have not yet exhausted the
non-flaky non-rendering layers (R1 + R2 + R3). **Defer `ui-vlm-judge`
to v0.2 of the test stack** — it's the right tool when widget-tree
+ kittest + coverage have all landed.

### R4 — Add `cockpit-smoke-live` orchestrator skill for the live-subscription path. [MEDIUM ROI]

**Effort**: 3 dev-days.

**Action**: pair the fixtures-only `cockpit-smoke` with a
live-mode variant that boots `cockpit_live` against a deterministic
synthetic-data fixture (the existing `live_subscription_full_bus.rs`
state) for 15-30 s, then asserts:

- Progress bar advances (frame N+5 differs from frame N)
- Spinner advances
- No `panicked at` in stderr
- At least one panel transitions from `Loading` to `Ready`

The 15-30 s window covers the multi-frame regressions the
fixtures-only smoke explicitly excludes (per
[`cockpit-smoke/SKILL.md § False-negative envelope`](../../.claude/skills/cockpit-smoke/SKILL.md)).

**Why this is fourth**: it's the last-mile coverage for the bug-#63
class but is more involved than R1 (which catches the same class
earlier in the pipeline at lower cost).

### R5 — Promote the silent-quarantine cases (`consistency::*`, `cockpit_live_kill_button_writes_audit`) to explicit `#[ignore = "see <slug>"]` or fix-them-now. [LOW EFFORT, IMMEDIATE]

**Effort**: 1 hour for the decision; 1-3 dev-days for the fixes
themselves.

**Action**: for each silent-failure-tolerated test:

1. Open a follow-up brief (`spec/<test-name>-quarantine/feature.md`).
2. Add `#[ignore = "tracked-in: <slug>"]` to the test.
3. Add the brief to `spec/backlog.md ## Queue`.
4. Now the test sweep is honestly green.

**Why this is fifth but should happen tomorrow**: the gate erosion
is real and is the most likely path to a future "tester reported PASS
but the operator hit a regression" incident. The fix is dirt-cheap.
The hard work is deciding whether to fix the violations or accept
them; either is acceptable, but silent tolerance is not.

### Recommended CI gate sequence (codified)

For the tester sub-agent, gate sequence:

```
1. scripts/precheck.sh                              (stdlib-shadow)
2. cargo fmt --all -- --check                       (rust-validate)
3. cargo clippy --workspace --all-targets --all-features -- -D warnings
4. cargo audit                                       (skip on offline runs)
5. cargo deny check
6. RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
7. scripts/check_no_clocks_in_ui_tests.sh
8. scripts/check_no_secrets_in_llm_artifacts.sh
9. uv run scripts/spec_lint.py
10. cargo test --workspace --lib -- --nocapture
11. cargo test --workspace --all-targets -- --nocapture
12. cargo test --workspace --doc
13. PROPTEST_CASES=1024 cargo test --workspace property_  (when proptest is touched)
14. scripts/verify_anchors.sh                       (when strategy/audit/exec/backtest/reports touched)
15. cargo llvm-cov --workspace --lcov ...           (R2 — new)
16. orchestrator-only: cockpit-smoke (fixtures)     (every UI brief)
17. orchestrator-only: cockpit-smoke-live           (R4 — new, every UI brief)
```

Steps 1-9 are gates (any failure blocks); steps 10-14 are core tests;
step 15 is warning-only for 2 weeks then gate; steps 16-17 are
orchestrator-only post-evaluator-PASS.

## §6 What this audit deliberately does NOT cover

- Strategic / product framing — that's the parallel
  `testing-strategy-review-2026-05-25.md` (analyst).
- Bench performance baseline — that's `rust-bench` skill territory.
- The 2026-05-15 deep-dive's pixel-vs-tree analysis at full depth
  ([ui-testability-deep-dive-2026-05-15.md](ui-testability-deep-dive-2026-05-15.md)) —
  this audit references it; the deep-dive remains the canonical
  document for the UI testing layer's reform.

## Changelog

- 2026-05-25 (architect): authored as read-only audit of the test
  framework topology + tooling. Triggered by Bug #63 R8 (stuck Lab
  progress bar) + the v3-vol-overlay no-op precedent
  (2026-05-22). Inventories 13 test layers, identifies 10 gaps,
  proposes 5 ranked recommendations led by R1 (subscription-pipe
  e2e canonical test class) + R2 (`cargo-llvm-cov` adoption).
  No code or spec changes; runs concurrently with the analyst's
  strategic review.
