---
slug: reflection-memory-trader-wiring
status: in-progress
owner: developer
updated: 2026-05-26
---

# Tasks — reflection-memory-trader-wiring

> Architect M-T1 pass complete 2026-05-26. R1-R7 + Q1-Q7 + K1-K8 +
> H1-H5 + 10-item non-regression contract from
> [feature.md](feature.md) ratified into ADR-0041 + this wave plan.
> Operator standing-Autoapprove applied to analyst-recommended
> defaults across Q1-Q7. Developer executes Waves A → B → C → D
> sequentially within waves and parallel where noted. Each row
> honours the honest-tick contract: Owner / Milestone / Depends on /
> Blocks / file:line / test cmd / expected output line.

## M0 — Analyst synthesis (DONE 2026-05-25)

_owner: analyst._

- [x] **T-AN-1** — Author `spec/reflection-memory-trader-wiring/feature.md`
      at v0.1.0 with R1-R7 + K1-K8 + H1-H5 + Q1-Q7 + 10-item
      non-regression contract.
- [x] **T-AN-2** — Append Active row to `spec/backlog.md`.
- [x] **T-AN-3** — Open trace row `REQ-REFLECTION-TRADER-001` at
      `proposed` state in `spec/trace.toml`.
- [x] **T-AN-4** — HANDOFF → architect for M-T1.

## M-T1 — Architect decomposition (DONE 2026-05-26)

_owner: architect._

- [x] **T-AR-1** (2026-05-26) — Ratify Q1-Q7 + R1-R7 by operator
      standing-Autoapprove on analyst-recommended defaults. Q1=(a)
      new `crates/trader/` + Q2=(a) clean-cut entire `llm_forecaster/`
      subtree + Q3=(a) no new trait (MemoryProvider deferred to
      v0.1.1) + Q4 mechanically subsumed by D4 registry-arm removal +
      Q5=(a) trader owns audit + Q6=(a)+(b) ship parallel with
      lab-yahoo-realdata v0.1.1 + Q7=(a) errata append.
- [x] **T-AR-2** (2026-05-26) — Move-set inventory (correcting two
      analyst miscounts; surfaced as contradictions in handoff
      report):
      - **9 source files** in `crates/strategy/src/llm_forecaster/`
        (analyst said 8): `mod.rs` (78 LoC), `trait_def.rs` (74),
        `types.rs` (1105), `canonicalize.rs` (98), `strategy.rs` (511),
        `anthropic_impl.rs` (672), `prompt.rs` (441), `tool_schema.rs`
        (255), `verdict.rs` (866) — total 4100 LoC.
      - **10 integration test suites** to move (analyst counted 13
        but that figure includes 3 non-strategy crates that stay put):
        `llm_forecaster_audit_tick.rs`, `llm_forecaster_budget_gate.rs`,
        `llm_forecaster_cost_cap_short_circuit.rs`,
        `llm_forecaster_cost_event.rs`, `llm_forecaster_neutrality.rs`,
        `llm_forecaster_payload.rs`, `llm_forecaster_signal_mapping.rs`,
        `llm_forecaster_wiremock.rs`, `llm_forecaster_wiremock_wave_e.rs`,
        `llm_verdict_priority_tree.rs`.
      - **1 strategy-crate binary** to move: `crates/strategy/src/bin/llm_verdict.rs`.
- [x] **T-AR-3** (2026-05-26) — Binary + library import-site
      inventory (`crates/ui/src/assistant/state.rs:21` doc-comment
      only — non-load-bearing reference; rewrite to
      `trader::llm_forecaster::types::LlmForecast`). Application
      binaries: `crates/ui/src/bin/cockpit_live.rs` +
      `crates/backtest/src/main.rs` (call `registry.load_from_toml`
      with `llm_forecaster_v3` kind — must add paired
      `trader::register_llm_forecaster_v3` call per ADR-0041 § D4).
- [x] **T-AR-4** (2026-05-26) — Registry-arm fate decided per
      [ADR-0041 § D4](../architecture/adr/0041-trader-crate-split.md):
      **full removal** from `crates/strategy/src/registry.rs`; new
      free function `pub fn register_llm_forecaster_v3` lives in
      `crates/trader/src/registry_arm.rs`; application binary
      assumes responsibility for both registration calls.
- [x] **T-AR-5** (2026-05-26) — Gate-test tightening decided per
      [ADR-0041 § D5](../architecture/adr/0041-trader-crate-split.md):
      **no list edit at v0.1.0**. Q4 mechanically subsumed by D4
      removal of the registry arm — once the arm leaves strategy,
      `NullReflectionStore` is structurally absent from strategy
      sources. Pre-emptive list tightening risks blocking a future
      legitimate test-double use; defer to a v0.1.1 follow-up if
      ever needed.
- [x] **T-AR-6** (2026-05-26) — `spec/v3-llm-forecaster/decomp.md`
      path-update strategy decided per ADR-0041 § D7: **leave
      historical evidence intact** (K6 mitigation option (iii)).
      The trace.toml `arch` column + this brief's feature.md +
      ADR-0041 carry the forward pointer.
- [x] **T-AR-7** (2026-05-26) — Author
      [`spec/architecture/adr/0041-trader-crate-split.md`](../architecture/adr/0041-trader-crate-split.md)
      with 6 top-level sections (Context / Decision / Alternatives /
      Consequences / References / Changelog). ADR registry row
      committed at [`spec/architecture/adr/README.md`](../architecture/adr/README.md)
      line 91.
- [x] **T-AR-8** (2026-05-26) — Cost re-estimate vs analyst
      strawman (3-5 days): **agreed** at 3.5-4.5 days wall-clock.
      Architect M-T1 ~0.5 d (DONE), developer M-DEV ~2-2.5 d (Wave
      A 0.5 d, Wave B 1-1.5 d, Wave C 0.5 d, Wave D 0.5 d), tester
      M-FINAL ~0.5 d, presenter ~0.5 d. The +0.5-1 d vs analyst's
      lower bound reflects the corrected move-set (9 source files
      + 10 test suites + 1 bin + new registry_arm.rs) and the
      additional UI doc-comment + application-binary wiring edits.
      No LLM costs (pure refactor).
- [x] **T-AR-9** (2026-05-26) — Populate `arch` column of trace
      row `REQ-REFLECTION-TRADER-001` with ADR-0041 + this
      tasks.md anchor. State stays at `proposed`; M-T1 progress
      captured inline in the `state` comment per the convention
      used by REQ-V3-LLM-FORECASTER-001 row 1072.
- [x] **T-AR-10** (2026-05-26) — HANDOFF → developer for M-DEV.

## M-DEV — Developer execution

_owner: developer. Wave-parallel where noted._

### Wave A — workspace plumbing (~0.5 day, blocks all)

> Sequential within wave. T-D-N1 must land + green before T-D-N2.

- [x] **T-D-N1** — Create `crates/trader/` skeleton.
  - Owner: developer • Milestone: M-DEV • Depends on: T-AR-10 • Blocks: T-D-N2..T-D-N9
  - File:line: `crates/trader/Cargo.toml:1` (new, ~30 LoC — package
    block + `[dependencies]` per [ADR-0041 § D1](../architecture/adr/0041-trader-crate-split.md):
    `trading-core` + `strategy` + `reflection` + `llm` + `audit`
    path-deps; dev-deps mirror existing
    `crates/strategy/Cargo.toml` llm_forecaster set —
    `rust_decimal`, `tokio` w/ `macros + rt-multi-thread + sync`,
    `async-trait`, `wiremock`, `serde_json`, etc.).
    `crates/trader/src/lib.rs:1` (new, ~5 LoC — empty placeholder;
    `pub mod llm_forecaster;` added in T-D-N3 once files land).
    `Cargo.toml` (workspace root) — additive: `"crates/trader"` in
    `[workspace.members]` alphabetical position (between
    `crates/strategy` and other later entries — confirm by
    eyeballing the existing list).
  - Body: at this step `lib.rs` is empty; Cargo.toml has the
    deps wired but the `pub mod` lines are commented out. The
    crate compiles green because there are no source files yet.
  - Test cmd: `cargo build -p trader`
  - Expected output: `Finished` (no compile errors; warnings about
    unused-dep are acceptable until Wave B lands the sources).
- [x] **T-D-N2** — Drop `reflection` path-dep from strategy crate.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N1 • Blocks: T-D-N3
  - File:line: `crates/strategy/Cargo.toml` — remove the line
    `reflection = { path = "../reflection" }` from `[dependencies]`
    (and any `[dev-dependencies]` entry referencing reflection's
    fake-store crate, if present). Remove the 9 `[[test]]` entries
    matching `llm_forecaster_*` + `llm_verdict_priority_tree` at
    lines 72-108 (they relocate to trader's Cargo.toml in T-D-N5).
    Remove any `[[bin]]` entry for `llm_verdict` if present
    (the strategy crate's bin moves in T-D-N4).
  - Body: at this step the strategy crate's source still imports
    `reflection::*` from `src/llm_forecaster/` and `src/registry.rs`,
    so `cargo build -p strategy` is EXPECTED TO FAIL with
    "unresolved import `reflection`". This is the load-bearing
    signal that the Wave B file moves are required before the
    workspace re-greens. **Do NOT stub the imports out**; let the
    next wave do the structural fix.
  - Test cmd: `cargo build -p strategy 2>&1 | tail -5` — RED is
    expected here.
  - Expected output: contains `error[E0432]: unresolved import
    \`reflection\`` (sentinel that confirms the dep drop landed).

### Wave B — module move (~1-2 days, depends on A)

> Sequential within wave. `git mv` to preserve blame on every file.
> Imports rewrite in the same commit as the moves so reviewers see
> one self-contained refactor commit.

- [x] **T-D-N3** — `git mv` the 9 source files.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N2 • Blocks: T-D-N4..T-D-N6
  - File:line:
    `git mv crates/strategy/src/llm_forecaster/mod.rs            crates/trader/src/llm_forecaster/mod.rs`
    `git mv crates/strategy/src/llm_forecaster/trait_def.rs      crates/trader/src/llm_forecaster/trait_def.rs`
    `git mv crates/strategy/src/llm_forecaster/types.rs          crates/trader/src/llm_forecaster/types.rs`
    `git mv crates/strategy/src/llm_forecaster/canonicalize.rs   crates/trader/src/llm_forecaster/canonicalize.rs`
    `git mv crates/strategy/src/llm_forecaster/strategy.rs       crates/trader/src/llm_forecaster/strategy.rs`
    `git mv crates/strategy/src/llm_forecaster/anthropic_impl.rs crates/trader/src/llm_forecaster/anthropic_impl.rs`
    `git mv crates/strategy/src/llm_forecaster/prompt.rs         crates/trader/src/llm_forecaster/prompt.rs`
    `git mv crates/strategy/src/llm_forecaster/tool_schema.rs    crates/trader/src/llm_forecaster/tool_schema.rs`
    `git mv crates/strategy/src/llm_forecaster/verdict.rs        crates/trader/src/llm_forecaster/verdict.rs`
    Add `pub mod llm_forecaster;` to `crates/trader/src/lib.rs`
    (uncommenting the placeholder from T-D-N1). Remove
    `pub mod llm_forecaster;` from `crates/strategy/src/lib.rs:19`
    + the doc-comment line at `lib.rs:15`.
  - Body: after the `git mv`, the moved files retain
    `use crate::*` references that point at strategy-crate
    symbols which no longer exist in their new home (e.g.
    references to `crate::Strategy` that resolved to
    `strategy::Strategy` before the move). T-D-N4 fixes these.
    Inside the moved files, `use reflection::*` imports remain —
    legitimate now because trader's Cargo.toml carries the
    `reflection` path-dep.
  - Test cmd: `ls crates/trader/src/llm_forecaster/`
  - Expected output: 9 file names (the moved set above) +
    `crates/strategy/src/llm_forecaster/` directory is gone.
- [x] **T-D-N4** — `git mv` the strategy-crate bin.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N3 • Blocks: T-D-N5
  - File:line:
    `git mv crates/strategy/src/bin/llm_verdict.rs crates/trader/src/bin/llm_verdict.rs`.
    Remove the `[[bin]] name = "llm_verdict"` entry from
    `crates/strategy/Cargo.toml` (if present) and add it to
    `crates/trader/Cargo.toml`. Rewrite the import inside
    `crates/trader/src/bin/llm_verdict.rs:53` from
    `use strategy::llm_forecaster::verdict::*` to
    `use trader::llm_forecaster::verdict::*`.
  - Body: the bin re-targets the trader crate's public API. If
    the bin pulls in other strategy-crate symbols (e.g.
    `strategy::SmaCrossover`), those imports stay valid because
    trader's Cargo.toml carries the `strategy` path-dep
    (ADR-0041 § D3 inverse-API).
  - Test cmd: `ls crates/trader/src/bin/llm_verdict.rs`
  - Expected output: file exists; `cargo build -p trader --bin
    llm_verdict` green after T-D-N6 lands the import rewrites.
- [x] **T-D-N5** — `git mv` the 10 integration test suites + move
      `[[test]]` entries in Cargo.toml.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N3 • Blocks: T-D-N6
  - File:line:
    `git mv crates/strategy/tests/llm_forecaster_audit_tick.rs            crates/trader/tests/`
    `git mv crates/strategy/tests/llm_forecaster_budget_gate.rs           crates/trader/tests/`
    `git mv crates/strategy/tests/llm_forecaster_cost_cap_short_circuit.rs crates/trader/tests/`
    `git mv crates/strategy/tests/llm_forecaster_cost_event.rs            crates/trader/tests/`
    `git mv crates/strategy/tests/llm_forecaster_neutrality.rs            crates/trader/tests/`
    `git mv crates/strategy/tests/llm_forecaster_payload.rs               crates/trader/tests/`
    `git mv crates/strategy/tests/llm_forecaster_signal_mapping.rs        crates/trader/tests/`
    `git mv crates/strategy/tests/llm_forecaster_wiremock.rs              crates/trader/tests/`
    `git mv crates/strategy/tests/llm_forecaster_wiremock_wave_e.rs       crates/trader/tests/`
    `git mv crates/strategy/tests/llm_verdict_priority_tree.rs            crates/trader/tests/`
    Move the 9 `[[test]]` blocks (lines 72-108 of the pre-T-D-N2
    `crates/strategy/Cargo.toml`) into `crates/trader/Cargo.toml`
    with paths rewritten from `tests/llm_forecaster_*.rs` to
    `tests/llm_forecaster_*.rs` (file basenames unchanged; only
    the host Cargo.toml differs).
  - Body: at this step `cargo build -p trader --tests` will FAIL
    on every test because each contains `use strategy::llm_forecaster::*`
    imports that no longer resolve (the symbols live in trader now,
    not strategy). T-D-N6 fixes this in one mechanical sweep.
  - Test cmd: `ls crates/trader/tests/ | grep -E '^llm_(forecaster|verdict)' | wc -l`
  - Expected output: `10` (exactly 10 moved test files; no
    leftovers in `crates/strategy/tests/`).
- [x] **T-D-N6** — Integration-test import-path rewrite + intra-crate
      `use crate::` audit in moved sources.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N5 • Blocks: T-D-N7
  - File:line: across all 10 moved test files + the moved bin +
    (if any) cross-file `use crate::*` refs in the 9 moved source
    files:
    1. `s/use strategy::llm_forecaster::/use trader::llm_forecaster::/g`
    2. `s/strategy::llm_forecaster::/trader::llm_forecaster::/g`
       (covers fully-qualified path references like
       `strategy::llm_forecaster::DEFAULT_MODEL_ID` in
       `llm_forecaster_payload.rs:383` etc. — see
       `grep -rn 'strategy::llm_forecaster' crates/` for the
       complete list pre-rewrite).
    3. Within the moved source files at
       `crates/trader/src/llm_forecaster/*.rs`, audit every
       `use crate::*` — references that pointed at strategy-crate
       symbols (e.g. `crate::Strategy`) now need to become
       `use strategy::Strategy` because trader is the consumer of
       the `Strategy` trait per ADR-0041 § D3. Run `grep -n
       'use crate::' crates/trader/src/llm_forecaster/*.rs`
       and reclassify each.
  - Body: post-rewrite, every `.rs` file in
    `crates/strategy/src/` is free of the substrings
    `strategy::llm_forecaster` (because the module is gone) AND
    the substring `reflection::` is structurally absent (the
    path-dep was dropped in T-D-N2). The gate-test t1809 may
    already PASS at this step but Wave D will run it formally.
  - Test cmd: `cargo build -p trader --tests` followed by
    `grep -r 'strategy::llm_forecaster' crates/ --include='*.rs'`
  - Expected output: build green; grep returns ZERO matches.
- [x] **T-D-N7** — Registry-arm extraction per ADR-0041 § D4.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N6 • Blocks: T-D-N8
  - File:line:
    1. Remove the `"llm_forecaster_v3"` match arm from
       `crates/strategy/src/registry.rs:130-146` (the arm + the
       `use reflection::NullReflectionStore` inside it).
    2. Remove the `if entry.kind == "llm_forecaster_v3"` skip-log
       block at `crates/strategy/src/registry.rs:111-119`.
    3. Remove the doc-comment line at `registry.rs:100`
       (`/// - "llm_forecaster_v3" — LLM-based directional…`).
    4. Remove the doc-comment lines 102-105 + 126-129 that
       reference the moved code paths.
    5. Create `crates/trader/src/registry_arm.rs` with the free
       function
       `pub fn register_llm_forecaster_v3(registry:
       &strategy::registry::StrategyRegistry, entry:
       &strategy::registry::StrategyTomlEntry) -> bool { … }`
       that contains the body lifted from the removed match arm.
       Return `true` if registered, `false` if skipped (per the
       enabled-flag + unknown-kind logic).
    6. Add `pub mod registry_arm;` to `crates/trader/src/lib.rs`
       and re-export the function at the crate root.
    7. In `crates/ui/src/bin/cockpit_live.rs` (and
       `crates/backtest/src/main.rs` if it also calls
       `load_from_toml` with `llm_forecaster_v3` entries; verify
       via `grep -n llm_forecaster_v3 crates/backtest/src/main.rs`),
       add a paired loop calling
       `trader::register_llm_forecaster_v3(&registry, &entry)`
       for entries with `kind == "llm_forecaster_v3"` BEFORE
       the existing `registry.load_from_toml(...)` call.
  - Body: structural enforcement of Q4 (no `NullReflectionStore`
    in strategy sources) without amending the gate-test list,
    per ADR-0041 § D5.
  - Test cmd: `grep -rn 'NullReflectionStore\|reflection::' crates/strategy/src/ --include='*.rs'`
    + `cargo build --workspace --bins`
  - Expected output: grep returns ZERO matches; build green
    across all binaries.

### Wave C — public-API exposure (~0.5 day, parallel-eligible with Wave B's tail)

> Parallel-safe with T-D-N7 once T-D-N6 lands. The lib.rs re-export
> work can start as soon as the source files are in their trader
> home (post T-D-N3).

- [x] **T-D-N8** — Re-export the public surface from
      `crates/trader/src/lib.rs`.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N6 • Blocks: T-D-N10
  - File:line: `crates/trader/src/lib.rs` — add:
    ```rust
    pub mod llm_forecaster;
    pub mod registry_arm;
    pub use llm_forecaster::{
        LlmForecaster, LlmForecasterImpl, LlmForecasterStrategy,
        LlmForecasterConfig, LlmForecasterError,
        ForecastContext, LlmForecast, Rating, Confidence, Horizon,
        StubForecaster,
        DEFAULT_MODEL_ID, DEFAULT_TIMEOUT_MS, DEFAULT_FIRE_EVERY_N_BARS,
        CACHE_SCHEMA_VERSION, PROMPT_TEMPLATE_VERSION, TOP_K_LESSONS,
    };
    pub use registry_arm::register_llm_forecaster_v3;
    ```
    Cross-reference against the pre-move
    `crates/strategy/src/llm_forecaster/mod.rs` re-export block to
    ensure byte-identical public API surface (R6.4 contract).
  - Body: the re-export surface IS the public API contract; any
    consumer (registry caller, integration test, UI assistant
    display) should be able to write
    `use trader::{LlmForecasterStrategy, LlmForecasterConfig}` and
    get the same types they used to get from
    `strategy::llm_forecaster::{...}`.
  - Test cmd: `cargo doc -p trader --no-deps 2>&1 | grep -c "pub"`
    (rough public-API smoke; tester locks the formal byte-identity
    gate at M-FINAL T-T-7 via `cargo-semver-checks` if available).
  - Expected output: non-zero `pub` symbol count;
    `cargo build -p trader` green.
- [x] **T-D-N9** — UI assistant doc-comment + crate Cargo.toml
      polish.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N8 • Blocks: T-D-N10
  - File:line:
    1. `crates/ui/src/assistant/state.rs:21` — doc-comment-only
       rewrite: `crates/strategy::llm_forecaster::types::LlmForecast`
       → `crates/trader::llm_forecaster::types::LlmForecast`. Non
       load-bearing (it's `//!`-prefixed). No `use` import change
       needed in this file.
    2. Verify no other `*.md`, `*.rs`, or doc-comment file in the
       repo references `strategy::llm_forecaster` post-rewrite:
       `grep -rn 'strategy::llm_forecaster' --include='*.rs'
       --include='*.md' .` — should hit only historical
       evidence files (spec/v3-llm-forecaster/decomp.md and
       similar reports). Those are deliberately left intact per
       T-AR-6 / ADR-0041 § D7.
    3. `crates/strategy/src/lib.rs:15` — remove the stale
       doc-comment line about `crates/strategy/src/llm_forecaster/`
       (it now points at a directory that doesn't exist).
  - Body: cleanup-only pass; no behavioural change.
  - Test cmd: `grep -rn 'strategy::llm_forecaster' crates/ --include='*.rs'`
  - Expected output: ZERO matches in `crates/` (historical files
    under `spec/` are out of scope per T-AR-6).

### Wave D — gate-test recovery (~0.5 day, depends on B+C)

> Sequential within wave. T-D-N10 must PASS before T-D-N11 lands.

- [x] **T-D-N10** — Verify gate-test t1809 flips RED → GREEN.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N8, T-D-N9 • Blocks: T-D-N11
  - File:line: no source change — observation only.
    `crates/reflection/tests/no_strategy_caller.rs::t1809_no_strategy_crate_consumes_reflection_retrieval`.
  - Body: the test walks `crates/strategy/src/` and asserts none
    of the 4 forbidden substrings appear. Post Wave B+C, every
    offending file has moved out of strategy; the test must
    PASS without code change. If it fails, root-cause is
    incomplete import rewrites (T-D-N6) or a missed
    reflection import in registry (T-D-N7) — fix before
    landing T-D-N11.
  - Test cmd: `cargo nextest run -p reflection --test no_strategy_caller -E 'test(t1809)'`
  - Expected output: `1 passed; 0 failed; 0 ignored; 0 filtered out;`
    + the test name `t1809_no_strategy_crate_consumes_reflection_retrieval`
    in the success list.
- [x] **T-D-N11** — Add positive-assertion sibling test t1810.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N10 • Blocks: T-D-N12
  - File:line: append to
    `crates/reflection/tests/no_strategy_caller.rs` (do NOT split
    into a new file — sibling-co-location per ADR-0041 § D5):
    ```rust
    #[test]
    fn t1810_trader_crate_owns_reflection_retrieval() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = manifest_dir
            .parent()
            .and_then(|p| p.parent())
            .expect("walk to workspace root");
        let trader_src = workspace_root.join("crates").join("trader").join("src");
        assert!(trader_src.exists(), "trader crate missing at {trader_src:?}");

        let required_substring = "reflection::retrieve_top_k";
        let mut found = false;
        for entry in WalkDir::new(&trader_src)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("rs"))
        {
            let src = match fs::read_to_string(entry.path()) {
                Ok(s) => s,
                Err(_) => continue,
            };
            if src.contains(required_substring) {
                found = true;
                break;
            }
        }
        assert!(
            found,
            "R5.3 / ADR-0041 § D5 — trader crate must own reflection retrieval; \
             expected at least one .rs file under {trader_src:?} to contain \
             `{required_substring}`. If memory retrieval is genuinely no longer \
             needed, write a superseding ADR removing both t1810 and the consumer."
        );
    }
    ```
  - Body: positive-assertion sibling guards against accidental
    deletion of the consumer logic during a future refactor.
    Shares the WalkDir + read_to_string + contains shape with
    t1809 so a future maintainer sees the contract as a sibling.
  - Test cmd: `cargo nextest run -p reflection --test no_strategy_caller -E 'test(t1810)'`
  - Expected output: `1 passed; 0 failed; 0 ignored; 0 filtered out;`
    + the test name `t1810_trader_crate_owns_reflection_retrieval`.
- [x] **T-D-N12** — Workspace re-green + anchor verification +
      handoff to tester.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N1..T-D-N11 • Blocks: M-FINAL
  - File:line: no source change — composite gate.
  - Body: run the full workspace test + anchor verification +
    binary build set locally before handing off. Document any
    flakes in `spec/reflection-memory-trader-wiring/reports/dev-final-2026-MM-DD.md`.
  - Test cmd:
    `cargo nextest run --workspace --no-fail-fast` then
    `cargo build --workspace --bins` then
    `bash scripts/verify_anchors.sh`
  - Expected output: workspace tests green (98 LLM-forecaster
    integration tests in the trader crate's count; sum of
    `passed` across all crates ≥ pre-refactor baseline); all
    binaries build green; `ANCHORS PASS (34 / 34)`.

#### Watch recipe for long-running tasks

The Wave B + Wave C combined `cargo nextest run -p trader` is the
longest-running step (~30-60 s on the LLM-forecaster integration
test set per the v3-llm-forecaster M-FINAL log). If running the
full workspace check in the background:

```sh
watch -n 5 'tail -n 30 /tmp/trader_refactor.log 2>/dev/null && \
  echo "---" && \
  pgrep -fl "cargo nextest"'
```

(Launch as `cargo nextest run --workspace --no-fail-fast 2>&1 \
  | tee /tmp/trader_refactor.log`.)

## M-FINAL — Tester verification

_owner: tester._

- [ ] **T-T-1** — Verify gate-test t1809 returns to PASS (R5.1 / H1).
  - Test cmd: `cargo nextest run -p reflection --test no_strategy_caller`
  - Expected output: both `t1809_no_strategy_crate_consumes_reflection_retrieval`
    AND `t1810_trader_crate_owns_reflection_retrieval` PASS.
- [ ] **T-T-2** — Verify R5.3 positive-assertion test t1810
      PASS in the same nextest invocation as T-T-1.
- [ ] **T-T-3** — Run `bash scripts/verify_anchors.sh`. Assert
      `ANCHORS PASS (34 / 34)` byte-identical (R6.1 / H2).
- [ ] **T-T-4** — Run `cargo nextest run -p trader`. Assert 98+
      LLM-forecaster integration tests PASS (R6.2 / H3). Compare
      pass count vs predecessor baseline from REQ-V3-LLM-FORECASTER-001
      `tests` column trace.toml line 1070 (98 across 13 listed
      paths; in this brief's scope 98 across 10 moved paths because
      the audit + UI suites that stayed put are now counted in
      their own crates).
- [ ] **T-T-5** — Run `cargo nextest run -p ui`. Assert 22+ Phase F
      visual snapshots + 11 layout invariants stay PASS (R6.3).
- [ ] **T-T-6** — Run `cargo build --workspace --bins` +
      `cargo run -p ui --bin cockpit_smoke`. Assert no binary
      regression + no cockpit panic (K4).
- [ ] **T-T-7** — Run
      `cargo metadata --format-version 1 | jq '.packages[] |
      select(.name == "strategy") | .dependencies[] | select(.name == "reflection")'`
      and assert EMPTY output (the strategy → reflection edge is
      gone per ADR-0041 § D1 / R4.3 / H4).
- [ ] **T-T-8** — Author test-final report at
      `spec/reflection-memory-trader-wiring/reports/test-final-2026-MM-DD.md`
      per `rust-test` skill template (8-row standard: verify_anchors /
      workspace / cockpit-smoke / clippy / fmt / criterion (N/A) /
      integration perf (N/A) / visual).
- [ ] **T-T-9** — VERDICT → PASS / REGRESSION. On PASS, flip trace
      row state → `passed` (matching the
      cockpit-activity-status-bar convention at line 1325) or
      `shipped` once presenter closes. On REGRESSION, route back
      to developer with the report.
- [ ] **T-T-10** — Populate `tests` + `crates` columns of trace row
      `REQ-REFLECTION-TRADER-001` once VERDICT → PASS. The
      `crates` column lists `crates/trader` (new), `crates/strategy`
      (modified — Cargo.toml + registry.rs + lib.rs), and
      `crates/reflection` (modified — no_strategy_caller.rs t1810
      addition).
- [ ] **T-T-11** — HANDOFF → presenter on PASS.

## M-PRESENTER — Sprint-review deck

_owner: presenter. Runs only after VERDICT → PASS._

- [ ] **T-P-1** — Author
      `spec/reflection-memory-trader-wiring/presentations/reflection-memory-trader-wiring-2026-MM-DD.md`
      per the standard presenter deck template. Sections: title
      slide / the operator-visible win (P0 gate-test red on `main`
      flipped green; workspace re-enters shippable state) /
      structural enforcement of R8.1 (strategy crate no longer
      links against reflection) / 34/34 anchors preserved /
      98/98 LLM-forecaster integration tests preserved / what's
      NOT in scope (Q3 `MemoryProvider` trait deferred to v0.1.1
      pending second consumer) / open questions (none — all 7 Qs
      operator-decided via standing Autoapprove) / risk register
      surfaced (K-vol-1 N/A; K6 historical evidence preservation
      done per T-AR-6) / verdict cell tree.
- [ ] **T-P-2** — Capture before-after evidence:
      - Before: `cargo nextest run -p reflection --test no_strategy_caller`
        output showing t1809 RED with the 3 offending substring
        citations.
      - After: same command showing both t1809 + t1810 PASS.
      - Before: `cargo tree -p strategy | grep reflection` showing
        the path-dep.
      - After: same command empty.
- [ ] **T-P-3** — Operator review. Capture verdict cell on H5
      ("3-5 day wall-clock") for the changelog (architect re-
      estimated to 3.5-4.5 d at T-AR-8).
- [ ] **T-P-4** — Operator approval → trace row state → `shipped`;
      backlog Active → Recent.

## T-OD — Operator-decide deltas surfaced by architect

> Architect M-T1 found no decisions the analyst's defaults DON'T
> cover. **Zero T-OD rows.** All 7 Q-questions stand at
> analyst-recommended defaults; load-bearing contradictions surfaced
> are file-count miscounts (T-AR-2) which are advisory — the actual
> move set is now corrected in the wave plan above and does not
> change the operator decision matrix.

## Notes

- **Parallelism map**:
  - Wave A sequential (T-D-N1 blocks T-D-N2).
  - Wave B sequential within wave (T-D-N3 → T-D-N4 + T-D-N5 → T-D-N6 → T-D-N7),
    blocked on Wave A.
  - Wave C parallel-safe with the tail of Wave B once T-D-N6 lands.
  - Wave D sequential, blocked on Wave B + Wave C tail.
  - M-FINAL gates parallel-safe across T-T-1..T-T-7 once developer
    hands off.
- **Anchor risk: ZERO by construction** (R6.1; pure package-level
  refactor; no scenario body bytes touched).
- **Predecessor**: `v3-llm-forecaster v0.1.0` (state =
  `shipped-partial`). This brief inherits the 98 integration test
  baseline + 34 anchor baseline.
- **Cost estimate**: 3.5-4.5 days wall-clock (architect ~0.5 d
  DONE, developer ~2-2.5 d, tester ~0.5 d, presenter ~0.5 d). No
  LLM costs.
- **Rollback cost**: ~10 commits across the 4 waves can be reverted
  in inverse order if M-FINAL reveals a load-bearing regression.
  `git mv` history preserves blame on revert; t1810 addition is a
  ~30 LoC append that reverts cleanly.

## Changelog

- 2026-05-25 (analyst): authored v0.1.0 stub at M0 close.
- 2026-05-26 (architect): M-T1 pass. Ratified analyst-recommended
  defaults on Q1-Q7 via operator standing-Autoapprove. Authored
  ADR-0041 (6 sections; 4 decisions D1-D7). Decomposed M-DEV into
  Waves A-D (12 T-D-N rows) + M-FINAL (11 T-T rows) + M-PRESENTER
  (4 T-P rows). Corrected analyst's file-count miscount (9 source
  files / 10 test suites / 1 bin — not 8 / 13). Cost re-estimated
  to 3.5-4.5 d. Zero T-OD operator-decide deltas. HANDOFF →
  developer for M-DEV.
- 2026-05-26 (developer): M-DEV complete. Waves A-D executed
  sequentially per plan. Created `crates/trader/` (Cargo.toml + lib.rs
  + registry_arm.rs). git-mv'd 9 source files + 1 bin + 10 integration
  tests from strategy → trader. Rewrote `use crate::Strategy` →
  `use strategy::Strategy` in moved strategy.rs; rewrote all 10 test
  files from `strategy::llm_forecaster` → `trader::llm_forecaster`.
  Removed registry arm + reflection dep from strategy crate.
  Added t1810 positive-assertion sibling to no_strategy_caller.rs.
  Updated reflection/src/lib.rs Q4 doc-comment + ui/assistant/state.rs
  doc-comment. Gates: t1809 RED→GREEN, t1810 GREEN, 34/34 anchors PASS,
  cargo build --workspace --all-targets GREEN. HANDOFF → tester.
