---
slug: claims-ledger-2026-08-15
status: living
owner: orchestrator
updated: 2026-08-15
---

# Claims ledger — what the project asserts vs. what the code does (2026-08-15)

A one-pass audit of the project's **capability and result claims** against the shipped
code, run after the 14-story adversarial review closed with nine disclosures
(`docs/dev-notes/bug-log.md` #74–#82; a tenth, #83, landed while this audit was running).

**Scope.** Sources audited: `README.md`, `CHANGELOG.md`,
`_bmad-output/planning-artifacts/PRD.md`,
`_bmad-output/planning-artifacts/architecture.md` (AD-1…AD-19), the 86-file ADR corpus,
`docs/runbooks/`, `docs/dev-notes/do-not-build-register.md`.

**Exclusion.** Nothing already disclosed as bug-log **#74–#83** is re-reported. Where a
claim is already covered there, the row says so and moves on.

**Provenance column** (rules of evidence — every row carries one):

| Tag | Meaning |
|---|---|
| `src` | I opened the cited file at the cited line this session. |
| `run` | I executed the gate/script this session and read its output. |
| `del` | A read-only delegated sweep reported it; I did **not** re-open the file. |
| `del+` | Delegated, and I re-opened the load-bearing line myself. |

---

## 1. Executive summary — the FALSE claims that matter, severest first

### S1. The CI "governance gates" step **cannot fail** on anchors — `run` + `src`

`.github/workflows/ci.yml:64-70` is the only remote enforcement of AD-2, the project's
most-cited invariant:

```yaml
      - name: Governance gates (anchors + spec-lint)
        if: matrix.os == 'ubuntu-latest'
        run: |
          bash scripts/verify_anchors.sh | tail -1
          python3 scripts/spec_lint.py
```

The step does not set `shell:`, so GitHub Actions runs it under its default `bash -e {0}`
— **`pipefail` is not enabled**. A pipeline's exit status is then `tail`'s, which is
always 0. Demonstrated locally this session:

```
$ bash -e -c 'false | tail -1';        echo $?   → 0
$ bash -eo pipefail -c 'false | tail -1'; echo $?   → 1
```

`scripts/verify_anchors.sh:3` documents that it "exits non-zero on any mismatch or missing
report" — that exit code is discarded. A corrupted anchor prints `FAIL …` into the log and
the job goes green.

**Not fatal, because**: `.githooks/pre-commit:13` *does* `set -euo pipefail` before the
same pipeline, and `git config core.hooksPath` is set to `.githooks` on this clone (`run`)
— so the gate binds locally at commit time. The remote gate is the inert one.
One-character fix: `shell: bash` on that step (or drop the `| tail -1`).

### S2. AD-16 is enforced by a **substring grep**, and the last overlay it blessed is wired to nothing — `src`

Three findings in one chain, and the third one is the product harm:

1. **The gate is textual.** `crates/strategy/tests/overlay_hygiene_gate.rs:54-76` requires
   only that a file named `<stem>_end_to_end.rs` exists and contains *one* substring from
   `{baseline_equity, quantity_scale, no-op signature, diverges, equity_diverge}` **and**
   one from `{".abs() >=", "assert_ne!", "!= 1.0", …}`. Nothing checks that an engine runs
   or that equity is compared. The CLAUDE.md non-negotiable it quotes verbatim at `:17-20`
   — "the overlay's **output equity** diverges from the un-targeted baseline equity" — is
   enforced by `grep`.
2. **A test was written to the scan.** `drawdown_control_overlay_end_to_end.rs:146` carries
   the comment *"Machine-recognizable form for `overlay_hygiene_gate`'s literal-pattern
   scan"* above an `assert_ne!` inserted for that purpose. The "LOAD-BEARING GATE"
   (`:109-173`) never touches an engine: it loops 10 fixture bars, sums
   `overlay.quantity_scale()` into `overlaid_cumulative_scale`, and compares it to
   `Decimal::from(equity_seq.len())` — a **literal 10**, labelled "Baseline run (no
   overlay)". It proves the overlay's own accessor returns something ≠ 1.
3. **Nothing in production calls it.** `DrawdownControlOverlay` appears in exactly three
   files: its own module, the `crates/strategy/src/lib.rs` re-export, and its own test
   (`rg DrawdownControlOverlay crates/ --glob '!crates/strategy/**'` → **0 hits**). The
   only production consumer of *any* `quantity_scale` is
   `crates/backtest/src/scenarios/garch_vol_target_overlay.rs:279` (the vol-targeting
   overlay). `CHANGELOG.md:84` claims the drawdown-control overlay "**ships with a day-1
   divergence e2e**"; `PRD.md:387` lists it in the shipped v2 tranche.

**This is the exact class AD-16 was written to prevent** — `scale` computed and never
applied — now on a third overlay, with the mandated gate passing. Note the reference
implementation AD-16 names, `vol_targeting_overlay_end_to_end.rs`, is also a
strategy-layer test (imports only `strategy::` and `trading_core::`, no engine); it is
honest about that in its own docstring, but it is not an equity-divergence e2e either.

### S3. AD-1's named identity test is a **determinism tautology** — `src`

`crates/backtest/src/bakeoff/scorecard.rs:1324` `scorecard_does_not_change_ranking`:

```rust
let ranking_before = rank_candidates(&candidates);
let _sc = compute_scorecard(&sharpes, &dummy_equity, 99);   // result discarded
let ranking_after  = rank_candidates(&candidates);          // same slice
assert_eq!(ranking_before.crowned, ranking_after.crowned, "crowned index changed!");
```

`rank_candidates(&[CandidateResult]) -> Ranking` (`bakeoff/rank.rs:44`) is a pure function
of its one argument; the scorecard is not reachable from it by construction. The test
proves `rank_candidates` is deterministic, which was never in question, and would stay
green under any change that *did* make ranking read a scorecard field. It is the same
shape the 1-21 review already rejected elsewhere ("the neutrality re-proof was a
determinism tautology") and the same shape as story 3-16's T-CAL finding — but this
instance is named in the architecture spine as AD-1's enforcement and has not been
disclosed.

**Its sibling binds.** `turnover_does_not_change_ranking`
(`crates/backtest/src/bakeoff/mod.rs:1612`) builds *two different* candidate sets
(turnover 0 vs 0.5/1.2/0.01) and asserts identical ranking — that one would go red if
ranking started reading turnover. AD-1 is therefore half-enforced, not un-enforced.

### S4. "Configurable 1–30d horizon (default 7)" is a hard-coded `7` — `del+`

`CHANGELOG.md:58` (advisor-forward-plan F6): *"…IF/THEN rules faithful to the real TOMLs
…; **configurable 1–30d horizon (default 7)**."*

The sole production producer is a literal:
`crates/agent/src/runtime.rs:1489` → `7, // default horizon_days (display-only)`.
The "range 1–30" exists only as prose in the struct doc,
`crates/ui/src/forward_plan/state.rs:314`. There is no `Message`, no config key, no
control, no clamp. Mitigating: the same doc comment says the value is
"**DISPLAY-ONLY** framing (ADR-0062 § D6); does NOT terminate the forward run" — so the
missing capability is a label, not a behaviour. The CHANGELOG line is still false as
written.

### S5. The data-quality **warning tier** can never fire — `del+`

`CHANGELOG.md:88` (advisor-data-quality-surface): *"…and **plain-language data-quality
warnings (thin liquidity / wash-trading / pump-and-dump)**."*

`crates/ui/src/leaderboard/state.rs:286-299`:

```rust
pub fn for_symbol(symbol: &str) -> Self {
    let _ = symbol;                     // "accepted (not currently branched on)"
    Self { venue: "Binance".into(), …, warnings: Vec::new() }
}
```

`warnings` is unconditionally empty; the three `DataQualityWarning` variants are
constructed only in `crates/ui/tests/leaderboard_data_quality_render.rs:204-206` (`del`).
The panel's other four elements (venue, provenance, trust, survival caveat) are real and
wired. Split verdict: **panel TRUE, warning tier FALSE.** The source comment is honest;
the CHANGELOG line is not.

### S6. AD-18 "registration is atomic" is enforced in **one direction only** — `run` + `src`

`scripts/adr_registry_check.py` exits 0 and its `--self-test` passes (`run`), but the
invariant it checks is *file → Registry row*. There is no row → file check.

Counted this session: **86 ADR files, 87 Registry rows.** The orphan is **ADR-0079**
("Shared σ̂ vol estimator (P1-5)") — a full row at
`_bmad-output/planning-artifacts/architecture/decisions/README.md:129` with no
`0079-*.md` anywhere in the tree. It is cited as a live dependency by
`0078-vol-targeting-overlay-reposition.md` ("Consumes: ADR-0079") and by production
source `crates/strategy/src/vol_estimator.rs:1` (`del`). AD-18 says "Numbers are never
reused"; here a number was *registered without a decision behind it*, which the lint
cannot see.

### S7. README's CI row is **inverted** — a hedge hiding a working capability — `src`

`README.md:73`: *"the GitHub Actions matrix stays **operator-parked** inert at
`.github/workflows/ci.yml.deferred` … (do not activate without operator direction)."*

Reality: `.github/workflows/ci.yml` exists and is live (`push`/`pull_request` on `main`).
No `.deferred` file exists. `architecture.md:245-251` (AD-13) correctly records the
2026-07-10 activation. The stale `DEFERRED — NOT ACTIVE` banner also survives *inside*
the live workflow at `ci.yml:1-9`, which is the more dangerous half: a reader who opens
the file is told it does not run.

Same paragraph, same staleness class: `README.md:14-17` says the burn-down has closed
"7 of 14" stories and names bug-log "#65-#69" as the disclosure set. Fourteen are closed
and the set is #65-#83.

### S8. AD-17's `SALT_TABLE` docstring guarantees something the shipped field violates — `src`

`crates/backtest/src/bakeoff/bootstrap.rs:57-62`:

> "Ensures two candidates in the same bake-off do **NOT** share resample draws even if
> their equity curves happen to be identical."

`:91` — `SALT_TABLE[candidate_index % SALT_TABLE.len()]` over a **16**-entry table
(`:63-80`). The shipped advisor field is 19 declared ids
(`crates/ui/src/leaderboard/runner.rs:55-61`) + the always-appended benchmark, i.e. 19–20
candidates. Indices 16+ wrap onto salts 0–3 and share a master seed with the earliest
arms. The spine **discloses this correctly** (`architecture.md:371-374`: "candidates ≥ 16
share salts with early arms"); the load-bearing code comment asserts the opposite
universal, and nothing asserts `field.len() <= SALT_TABLE.len()`.

### S9. AD-14(c)'s universal is false on the instrument AD-14 itself names — `run` + `src`

AD-14(c): *"**`ui` (lib + every bin) never depends on `strategy`, `exec`, or `forecast`**
— unconditionally, in any build or feature set … **`cargo tree -p ui` unchanged is the
per-change gate**."*

`cargo tree -p ui` (run this session) lists `exec v0.1.0`, `strategy v0.1.0` and
`forecast v0.1.0`, reached through `backtest` and `agent`. The mermaid diagram at
`architecture.md:291-319` draws `ui --x strategy` (forbidden) in the same graph as
`ui --> backtest` and `backtest --> strategy`.

The *narrow* claim holds and is worth keeping: `crates/ui/Cargo.toml` has **no** direct
`strategy`/`exec`/`forecast` edge (verified; the only mention is the explanatory comment
at `:328`). What is false is the "unconditionally, in any build or feature set" wording
and the choice of `cargo tree` as its gate. AD-14 itself concedes the enforcement is
"**not automated**".

### S10. The short-selling product surface is **unreachable from the operator's journey** — `src`

`BakeoffConfig::default_short_field()` (`crates/backtest/src/bakeoff/mod.rs:701`) has
**zero production callers** — its only caller is
`crates/backtest/tests/p2_verdict_rerun.rs:212`. `advisor_field()`
(`crates/ui/src/leaderboard/runner.rs:55-61`) concatenates `default_field` +
`default_ensemble_field` + `default_macro_field` only.

Consequence: `is_short_capable_id` (`crates/ui/src/screens/leaderboard.rs:1610`), the
short pill, and the load-bearing unbounded-loss disclaimer gated by `field_has_short_arm`
(`:519-526`) can never be true in the shipped cockpit.

**Read this carefully — it cuts two ways.** `CHANGELOG.md:65` is *explicitly honest*
about the design ("a SEPARATE `default_short_field` (the standard 13-arm advisor stays
long-only)") — that line is TRUE and should be credited. What is overstated is the same
line's "the UI shows shorts + honest NEGATIVE P&L + a 'can lose more than your €200'
disclaimer": that UI exists and is pixel-proven against a synthetic report, but the
operator's journey cannot produce a report that renders it. And it is a **correction to
the framing of #80/#82**, which describe the short slate as "the ranked field the
operator reads" — on the cockpit path the operator never runs those arms at all. The
measurement in #82 stands; its product-harm framing applies to the research lane.

### S11. CI's test slice compiles out every feature-gated test — `src`

`.github/workflows/ci.yml:115` runs `cargo test --workspace --exclude ui` with **default
features**. `crates/backtest/Cargo.toml` declares **no** `default = [...]`: `realdata`,
`candle` and `yahoo` are all opt-in. Sixteen integration-test files carry
`#[cfg(feature = "realdata"|"candle"|"yahoo")]` gates (~105 gate sites across tests and
`src`), including the real-Binance behavioural determinism anchors
(`crates/backtest/tests/determinism.rs:889+`), `p2_verdict_rerun.rs`, and the
lot-realism / DVOL end-to-end suites. None of them compile in CI. Twelve ungated
behavioural anchor re-derivations (`determinism.rs:502-710`) *do* run there.

Also absent from CI entirely: `cargo fmt --check`, `cargo clippy -- -D warnings`,
`rust-validate`, and the AD-5 PIT lint — all four are named in AD-19's release floor.

### S12. Two more "declared, not executed" capability lines — `del`

- **`SlippageModel::VolScaledSpread`** (`crates/cost/src/slippage.rs:113`) — the "opt-in
  cost-model variant" of the v2 tranche has zero production construction sites; the CLI
  builder can only emit `SquareRoot`/`Linear` (`crates/backtest/src/main.rs:168-181`) and
  the advisor path uses `LatencySlippageSimConfig::advisor_default()`
  (`crates/backtest/src/cli_types.rs:137-142`), leaving `Linear{bps:0}`.
  `fee_sensitivity_report` (`slippage.rs:282`) has no callers outside its own tests.
- **The R1 forward-buildability line is now stale.** The CHANGELOG's "all 14 post-F5b
  arms are forward-buildable … no longer `bail!`s" was made false by #81's own (correct)
  remediation: `v0.macro_riskon` now deliberately `anyhow::bail!`s at
  `crates/agent/src/runtime.rs:640` (`src` — I read this). The fix was right; the index
  line did not move with it.

### S13. PRD "registry identity" is true for 17 arms, not all — `src`

`PRD.md:262`: *"the forward-run strategy is **the same artifact** the bake-off ranked
(anti-fake gate: registry-identity + divergence proof vs a proxy); an unknown id yields a
typed error, never a silent fallback."*

`crates/agent/src/runtime.rs:596-609`: `v0.dvol_regime` is registered for the forward run
with `vec![]` as its as-of series — *"empty as_of → permanent warm-up → hold the coin"*.
Same **type**, different **artifact**: an arm ranked on real DVOL gating forward-runs as
buy-and-hold under its own label. The code comment is candid about this and explains why
(review 3-15 made the description and the behaviour agree); the PRD sentence is the
overclaim. `v0.macro_riskon` now bails (S12) rather than substituting — which is the
honest rendering and *strengthens* the second half of the PRD sentence.

---

## 2. The AD-1 … AD-19 invariant table

Question asked of every row: **would the named code actually go red if the invariant were
violated?**

| AD | Invariant | Named enforcement | Binds? | Evidence |
|---|---|---|---|---|
| **AD-1** | FROZEN gate byte-frozen; additions prove ranking-neutrality | 2 identity tests | **HALF** | `turnover_does_not_change_ranking` (`bakeoff/mod.rs:1612`) varies input → binds. `scorecard_does_not_change_ranking` (`scorecard.rs:1324`) calls `rank_candidates` twice on one slice → tautology (S3). No mechanical freeze on the 4 functions themselves — review-only. `src` |
| **AD-2** | Anchors 119/119 byte-identical before **and** after | `scripts/verify_anchors.sh` (local + CI) | **LOCAL yes / CI no** | Ran it: `ANCHORS PASS (119 / 119)`; `evidence/anchors.toml` has exactly 119 rows. Binds via `.githooks/pre-commit` (`pipefail` set). CI step's exit code is swallowed (S1). Scope note: the script re-hashes **stored files**; behavioural re-derivation lives in `determinism.rs` (16 scenarios, 12 of them ungated). `run`+`src` |
| **AD-3** | Advisor path `write_report=false`; every knob defaults byte-identical | `venue_filter_default_is_none`, `paper_step_none_is_byte_identical` | **YES, narrowly** | `bakeoff/mod.rs:1258` `write_report: false`. Both named tests exist (`backtest/src/paper.rs:375,390`) and are default-value assertions — which is exactly what their ADR claims (ADR-0081 §84), so no overclaim. Caveat: anchor-neutrality ≠ liveness — #79 showed a knob that was both byte-neutral and totally inert. `src`+`del` |
| **AD-4** | Story `Status:` ↔ trace ↔ CHANGELOG move together | `scripts/spec_lint.py` | **YES** | `spec_lint.py` → `PASS (0 violations)`; `--self-test` → all three rules "fire on drift, silent on compliant". **Stale reference:** AD-4 names rules `feature-shipped-trace-drift` / `feature-shipped-changelog-missing`; the shipped rule ids are `status-drift` / `story-done-trace-drift` / `story-done-changelog-missing` (`spec_lint.py:112-114`). `run`+`src` |
| **AD-5** | PIT discipline structural + linted | `check_no_raw_asof_join.sh` + trybuild | **YES — one of the strongest gates in the repo** | Ran both: `SELF-TEST PASS: offending fixture flagged (1 hit), clean fixture silent (0 hits)`, then `PIT-JOIN LINT PASS (scanned 405 production src files)`. `AsOf<T>` fields are private (`crates/core/src/pit.rs:56-59`), constructed only in `PitSeries::as_of` (`:239-246`); a committed trybuild compile-fail guards it (`del`). Not run by CI (S11). `run`+`src` |
| **AD-6** | Benchmark exempt from the fragility gate | `BenchmarkWins`-reachability test | **YES** | `rank.rs:71-76` filters `!c.is_benchmark` out of `all_active_fragile`; `is_eligible` at `:151-153`; reachability test at `crates/backtest/tests/robustness_bootstrap_bites.rs:544`. `src` |
| **AD-7** | Narration passes faithfulness or falls back; LLM never ranks | `narration_faithfulness.rs` corpus | **YES** | Production path gates on `check_faithful` (`crates/agent/narration.rs:875`); `BANNED_PHRASES` at `:337` checked at `:578`; P3 verbatim-number match at `:701`. ADR-0064's *other* claimed gate — "no `llm` type reaches a `view`" — has **no automation**; the invariant currently holds in fact (grep: 0 hits) but nothing would catch a regression. `del` |
| **AD-8** | Additive-only; 3 seams; every crownable arm forward-buildable | per-family `builds_not_bails` tests | **MOSTLY** | `crates/agent/tests/forward_run_engine_fidelity.rs` is *stronger* than its ADR's wording: `:72-161` assert the registered identity is not the SMA fallback, `:189-332` compare signal streams. The exhaustive every-id-resolves test is declared Deferred in the spine. #81 was the hole this left. `del` |
| **AD-9** | Money is `Money<C>`/Decimal, never f64 | type system + reconciler | **YES** | `crates/core/src/money.rs:15` — Decimal-backed. The one f64 in the sizing chain (`Strategy::quantity_scale`) is converted explicitly and defensively at `scenarios/garch_vol_target_overlay.rs:279-284`. `src` |
| **AD-10** | UI proven at the rendered-PIXEL layer | 33 `*_render.rs` harnesses | **YES, with a hole** | 33 render/snapshot files, **363** `#[test]`, **36** `#[ignore]`d (~10%). The ignored set is the *populated shell* group — `positions_ready`, `agent_feed_ready`, `kpi_strip_ready`, `pnl_panel_ready`, `focus_ring_baseline` × 3 viewports (`crates/ui/tests/render_snapshots.rs:133-394`), reasons honestly stated (spinner/uptime non-determinism). CI's macOS leg runs `cargo test -p ui` and never passes `--ignored`, so they run only when an operator remembers. `src` |
| **AD-11** | do-not-build register binding; thesis era-qualified | "operator review against the register" | **NO MECHANISM (declared)** | The register exists (`docs/dev-notes/do-not-build-register.md`, 205 lines) and the era-qualified wording is used consistently in README/PRD/architecture (spot-checked). Enforcement is human by design — the AD says so. `src` |
| **AD-12** | DSR report-only; crown-veto unbuilt | ranking-identity tests + the unbuilt veto | **YES** | `grep -c "scorecard\|dsr\|DSR" crates/backtest/src/bakeoff/rank.rs` → **0**. The ranking cannot read it. (The AD-1 identity test that nominally co-guards this is S3's tautology, but the structural fact is independently verifiable.) `run`+`src` |
| **AD-13** | 3-OS CI active; macOS canonical visual box | live workflow + `cfg(target_os)` | **YES** (README contradicts it) | `.github/workflows/ci.yml` exists and triggers on push/PR. macOS leg runs the full `-p ui` suite; `pub mod fixtures;` is unconditional (`crates/ui/src/lib.rs:113`), so the visual tests do compile there without `--features fixtures`. 56 baseline PNGs present, matching the workflow comment. README says the opposite (S7). `src` |
| **AD-14** | Dependency-direction law; `ui` never depends on strategy/exec/forecast | review + manual `cargo tree -p ui` | **NARROW yes / stated form NO** | Direct manifest edges absent (verified). Transitive edges present in `cargo tree -p ui` (S9). The AD concedes "not automated". `run`+`src` |
| **AD-15** | PAPER/SIM only — no live execution path | absence of a venue write client | **YES** | `rg -i "hmac\|signature=\|X-MBX-APIKEY\|secret_key"` over non-test `crates/` → one hit, in an LLM *redaction* test helper. No `place_order`/`submit_order`/`create_order`. Export is a file serialiser (`screens/forward_plan.rs:1139`, `del`). `run`+`src` |
| **AD-16** | Day-1 baseline-equity-divergence e2e for every overlay/sizing modifier | `overlay_hygiene_gate` + per-feature `*_end_to_end.rs` | **NO** | S2. Additional scope gap: the gate globs `crates/strategy/src/*_overlay.rs` only — `patchtst_overlay_momentum.rs` and `tcn_overlay_momentum.rs` do not match the suffix and are never scanned (both retired), and **sizing modifiers outside `crates/strategy` are out of scope entirely** (`crates/risk/src/sizing.rs`). The risk-side sizing e2e nevertheless exists and binds — see §5. `src` |
| **AD-17** | Determinism envelope; positional per-arm salts | `check_determinism_anchors.py` + determinism tests | **YES, with a false code comment** | `derive_master_seed` = `wrapping_add` (`bootstrap.rs:92`), which the spine already records as diverging from ADR-0063's "XOR" — correctly handled as an as-built divergence. The salt-collision issue is disclosed in the spine and denied in the code comment (S8). `src` |
| **AD-18** | ADR + Registry row in the same commit; numbers never reused | `scripts/adr_registry_check.py` | **HALF** | One-way (file → row). 86 files vs 87 rows; ADR-0079 is a row with no decision (S6). `run`+`src` |
| **AD-19** | Release floor: fmt + clippy + rust-validate + anchors + spec_lint | gate scripts + tester workflow | **PARTIAL** | Of the five, CI runs `spec_lint` (binds) and `verify_anchors` (inert, S1) and **none** of fmt/clippy/rust-validate. `.githooks/pre-commit` runs `spec_lint` + `verify_anchors` (both bind). fmt, clippy and the PIT lint remain discipline-only — the precise condition the hook's own header cites bug-log #66 for. `src` |

---

## 3. Product-capability claims (what the product does for the operator)

| # | Claim (source) | Verdict | Evidence | Prov. |
|---|---|---|---|---|
| P1 | "configurable 1–30d horizon (default 7)" — `CHANGELOG.md:58` | **FALSE** (label only) | Literal `7` at `crates/agent/src/runtime.rs:1489`; range is prose at `crates/ui/src/forward_plan/state.rs:314` | `del+` |
| P2 | "plain-language data-quality warnings (thin liquidity / wash-trading / pump-and-dump)" — `CHANGELOG.md:88` | **FALSE** (panel TRUE, warnings FALSE) | `warnings: Vec::new()` unconditional, `crates/ui/src/leaderboard/state.rs:299` | `del+` |
| P3 | "a drawdown-control sizing overlay … ships with a day-1 divergence e2e" — `CHANGELOG.md:84` | **FALSE** | Zero production consumers; test never runs an engine (S2) | `src` |
| P4 | "opt-in `VolScaledSpread` cost variant + fee-sensitivity read" — v2 tranche | **PRESENT, UNREACHABLE** | No production construction site; advisor path pins `Linear{bps:0}` | `del` |
| P5 | "the UI shows shorts + negative P&L + unbounded-loss disclaimer" — `CHANGELOG.md:65` | **PRESENT, UNREACHABLE** | `default_short_field()` has no production caller (S10) | `src` |
| P6 | "the standard advisor stays long-only; shorts live in a SEPARATE field" — `CHANGELOG.md:65` | **TRUE** | Same evidence as P5, read the other way — the design statement is accurate | `src` |
| P7 | "all 14 post-F5b arms forward-buildable; no longer `bail!`s" — R1 line | **NOW FALSE** | `v0.macro_riskon` bails deliberately, `crates/agent/src/runtime.rs:640` (correct fix, unmoved index line) | `src` |
| P8 | "the forward-run strategy is the same artifact the bake-off ranked" — `PRD.md:262` | **PARTLY FALSE** | `v0.dvol_regime` registers `vec![]` as-of → holds the coin (`runtime.rs:596-609`) | `src` |
| P9 | "Budget … enforced as a hard sizing cap"; "paper sizing never deploys more than the budget" — `PRD.md:104,268` | **TRUE** | `with_budget_cap` wired at `crates/agent/src/runtime.rs:2049`; initial capital = budget at `:2066-2071`; clamp at `crates/risk/src/sizing.rs:77-82`. See §5 for *why* it holds (the mechanism is not the cap) | `src` |
| P10 | "EUR→USDT conversion … €200 ≈ $216.00 (at 1.08, config)" — F7 | **TRUE / WIRED** | Config-resolved `crates/ui/src/bin/cockpit_live.rs:458-462`; single multiply `BudgetConversion::new`; feeds `FixedFractionSizer::with_budget_cap` | `del` |
| P11 | "on-demand fetch for any coin + window" | **TRUE / WIRED** | `resolve_bakeoff_bars` (`bakeoff/mod.rs:1048`) falls through to `data::dynamic_cache::load_or_fetch` (`:528`); `dynamic_cache` is un-gated, so it ships | `del` |
| P12 | "Ready-only 'Export this plan' button writes `plan-exports/…`" — P5/ADR-0088 | **TRUE / WIRED** | Button gated on `Ready` (`screens/forward_plan.rs:237-239,269`); single `fs::write` leaf at `:1139` | `del` |
| P13 | "report-only overfitting scorecard; `rank.rs` never reads it" | **TRUE** | 0 references in `rank.rs` (§2 AD-12) | `run` |
| P14 | "Churn column + Risk-story tail block on the leaderboard" | **TRUE / WIRED** | turnover `bakeoff/mod.rs:949` → mirror → `screens/leaderboard.rs:1476`; `crown_tail` under Bootstrap, which the advisor forces (`runner.rs:123-128`) | `del` |
| P15 | "FRAGILE tuned config's 'Use this config' is locked" — UJ-2 | **TRUE / WIRED** | `promotable = !matches!(verdict, Fragile)` (`crates/ui/src/tune/state.rs:308`) gates the `on_press` | `del` |
| P16 | "DATA → CALIBRATE → ANALYZE → SUGGEST stepper tracks substate" — ADR-0083 | **TRUE / WIRED** | `stage_stepper::stage_for(...)` reads `PanelState`, pushed in `shell::view` (`crates/ui/src/shell.rs:61-63,91`), the entry point of both binaries | `del` |
| P17 | "the advisor honestly reports the arm count for this coin/build" | **TRUE** | `advisor_field_arm_count[_for]` routes through the same `arm_runs_in_this_build` / `dvol_supported` predicates the loop uses (`crates/ui/src/leaderboard/runner.rs:84-116`) — this is the #81 remediation and it is well built | `src` |

---

## 4. Verification claims ("all green", "N tests pass", "gate PASS")

| # | Claim | Verdict | Evidence | Prov. |
|---|---|---|---|---|
| V1 | "119/119 anchored body-SHAs byte-identical" — `README.md:72` | **TRUE** | Ran `scripts/verify_anchors.sh` → `ANCHORS PASS (119 / 119)`; 119 rows in `evidence/anchors.toml` | `run` |
| V2 | "the anchor gate is CI-enforced" — implied by AD-2 and `ci.yml`'s step name | **FALSE for CI** | Exit code discarded (S1) | `run` |
| V3 | "full lib/integration/UI-snapshot suite green" — `README.md:72` | **UNVERIFIED + scope-dependent** | Not run this session (cost). Scope caveats are load-bearing: **77** real `#[ignore]` attributes workspace-wide; 16 test files behind `realdata`/`candle`/`yahoo`, none of which CI compiles (S11); 36 of 363 render tests ignored | `src` |
| V4 | "spec-lint PASS" | **TRUE and self-verified** | `spec_lint.py` → `PASS (0 violations)`; `--self-test` proves all three triad rules fire on drift and stay silent on compliant input | `run` |
| V5 | "ADR registry check passes" | **TRUE but one-way** | Exit 0, self-test OK; cannot see an orphan row (S6) | `run` |
| V6 | "PIT look-ahead lint PASS" | **TRUE and self-verified** | 405 files scanned; `--self-test` flags a planted violation | `run` |
| V7 | "the constitution is MACHINE-enforced (CI gates + `.githooks`)" — 2026-07-27 hardening | **HALF** | Pre-commit binds (spec_lint + anchors, `pipefail` set, `core.hooksPath` configured). CI binds spec_lint only | `run`+`src` |
| V8 | "56 visual baselines … the canonical visual gate" — `ci.yml:106-110` | **TRUE** | 56 PNGs under `crates/ui/tests/visual-baselines`; `pub mod fixtures` unconditional so they compile on the macOS leg | `src` |
| V9 | ADR-0051: "goes RED the instant any short statement leaks out of its gate"; "the `funding_override_none_anchor_neutrality` test at `montecarlo.rs:648`" | **STALE + VACUOUS** | Named symbols do not exist; the shipped `run_path_funding_none_is_anchor_neutral` (`montecarlo.rs:1389`) calls `run(None)` twice and compares — a leak corrupts both runs identically; `:1398` silently `return`s (passes) if the TOML can't be resolved. ADR never amended | `del` |
| V10 | ADR-0039: "`cargo test -p strategy --test llm_forecaster_verdict_mutual_exclusivity`" | **MISSING** | No such file or symbol anywhere in `crates/`; ADR status `accepted` | `del` |
| V11 | ADR-0034: "asserts the output matches a checked-in golden … `insta::assert_snapshot!`" | **DOWNGRADED** | Shipped `train_tcn_golden_cli.rs:12-14` uses keyword assertions, explicitly "not a byte-exact snapshot"; a new flag or changed default passes | `del` |
| V12 | ADR-0065: "the multiply is in exactly one place" | **TAUTOLOGY** | `crates/core/tests/eur_fx_conversion_applied.rs:143-161` re-performs the multiply it claims to police; the "real grep guard … in CI/precheck" is absent from `scripts/precheck.sh` | `del` |
| V13 | ADR-0004: "Test coverage: `crates/audit/tests/journal.rs::test_microsecond_ts`" | **STALE PATH, coverage exists** | No such file; equivalent assertions live in `feed_reconnect_test.rs:77`, `kill_switch_dual_write_test.rs:184`, `uptime_intervals_test.rs:143` | `del` |

---

## 5. Claims verified TRUE — what the project can legitimately stand behind

These were checked adversarially and held. This section is as much the deliverable as §1.

1. **The anchor corpus and its gate.** 119 rows, 119 passes, a 247-line resolver that
   handles five namespace generations, and a real body-SHA re-hash. It binds at every
   local commit through `.githooks/pre-commit` (`pipefail` set, `core.hooksPath`
   configured). Fix S1 and it binds remotely too. `run`

2. **The PIT / as-of layer — the best-engineered gate in the repo.** `AsOf<T>` has private
   fields and exactly one constructor (`crates/core/src/pit.rs:56-59, 239-246`), a
   committed trybuild compile-fail guards the invariant, and the backstop lint
   (`check_no_raw_asof_join.sh`) matches the *predicate shape* rather than a symbol name,
   carries a `// PIT-OK:` escape hatch, and ships a self-test that plants a violation and
   proves the matcher catches it. It scans 405 production files clean. `run`+`src`

3. **The spec-lint triad.** Three rules, all with self-tests that prove *fire-on-drift and
   silence-on-compliant* — the shape most of the repo's other "gates" lack. `run`

4. **The frozen-gate report-only contract.** `rank.rs` contains zero references to the
   scorecard or DSR. AD-12's "never a veto" is structurally true, not merely asserted.
   `run`

5. **Benchmark exemption (AD-6).** `rank.rs:71-76` really does exclude the benchmark from
   the `AllFragile` determination, with a reachability test behind it. `src`

6. **PAPER/SIM only (AD-15).** No signing, no API-key header, no order-placement symbol
   anywhere in non-test code. The one "hand-off" is a file write. `run`

7. **The €200 budget cap holds on the shipped path — but know *why*.** The forward loop
   sets initial capital to the budget and builds
   `FixedFractionSizer::with_budget_cap(fraction, budget)`
   (`crates/agent/src/runtime.rs:2044-2071`). Two honesty notes, both verified at source:
   *(a)* both caps in `sizing.rs:60-82` are **per-order** clamps (`qty·price ≤ budget`),
   and `Order::new`'s exposure cap (`crates/core/src/order.rs:160-167`) is
   `notional/equity` for **this order only** — neither is a position-level cap; *(b)* what
   actually prevents accumulation is the long-only clamp
   `SignalKind::Buy if position.base_qty <= 0` (`runtime.rs:2175`, and its bake-off twin
   `scenarios/sma_composed_run.rs:570,1140`). Since the shipped advisor field is entirely
   long-only, the claim holds. It is the *same code* with `short_enabled = true` that
   bug-log #82 measured ratcheting to 11–16× — so the guarantee rests on the field
   composition, not on the cap. `src`

8. **The AD-16 gate that *does* bind.** `crates/risk/tests/budget_sizing_divergence_end_to_end.rs:151`
   calls the production `FixedFractionSizer::compute_qty` on both arms with only the cap
   differing — a no-op clamp gives divergence 0 and the test goes red. It also states
   plainly (`:106`) that it does not claim to prove the live loop uses `with_budget_cap`.
   That is the honest shape the drawdown gate should have copied. `del`

9. **Forward-run engine fidelity (ADR-0077).** `crates/agent/tests/forward_run_engine_fidelity.rs`
   asserts each arm's registered identity is not the SMA fallback (`:72-161`) and compares
   signal streams between registries on identical bars (`:189-332`) — materially stronger
   than the ADR's own modest wording. `del`

10. **The `#81` remediation is well built.** The single predicate `arm_runs_in_this_build`
    is read by *both* the dispatch loop and the cockpit's arm count
    (`crates/ui/src/leaderboard/runner.rs:84-116`), so the number on screen cannot claim
    an arm the loop drops — and the guard is kept live (rather than emptying the field) so
    it stays provable. The `v0.macro_riskon` forward-loop `bail!`
    (`crates/agent/src/runtime.rs:640-649`) carries a full explanation of why substituting
    was worse than refusing. `src`

11. **`crates/reflection/tests/no_strategy_caller.rs`** pairs a negative scan with a
    *positive* sibling over `crates/trader/src/`, so the negative cannot pass vacuously on
    an empty directory. `del`

12. **CHANGELOG:65's design statement** ("the standard advisor stays long-only") and the
    **era-qualified thesis wording** in README/PRD/architecture were both checked for the
    forbidden universal form and are consistently qualified. `src`

13. **AD-3's default-is-byte-identical tests match their ADR's wording exactly**
    (ADR-0081 §84 claims a default-value assertion and ships one) — claim and test agree;
    no overclaim, despite the superficial resemblance to the #79 shape. `del`

---

## 6. Could not determine, and why

| Question | Why unresolved |
|---|---|
| Is "the full suite green" actually true today? | Never run — a full `cargo test --workspace --all-features` plus the 77 `#[ignore]`d tests is a multi-hour job outside this audit's read-only budget. The *scope* caveats in V3 are established; the *result* is not. |
| Do the 36 ignored render tests still pass under `--ignored`? | Same reason. Their ignore reasons (spinner/uptime non-determinism) are plausible and documented, but unverified. |
| Are the numeric results quoted in `docs/runbooks/advisor-end-to-end-demo.md` (e.g. buy-and-hold +47.78%) reproducible today? | Requires real backtest runs against the pinned corpus; out of scope for a read-only pass. The runbook's *structural* claims (19 candidates, macro/short in separate fields) do match the code. |
| Does `data/deribit-dvol` / `data/yahoo-macro` cover every window the advisor can request? | Data-dependent and window-dependent; this is the open half of bug-log #78 and is already tracked there. |
| Is ADR-0079's content recoverable, or was the decision never written? | `git log --all` was reported clean for the file (`del`); I did not independently re-run the history search. Either way the Registry row is unbacked today. |
| Would `cargo clippy -- -D warnings` pass right now? | Not run (cold-cache clippy over 17 crates × 3 feature sets). Note the standing repo lesson that clippy's cache "lies both ways" — a claim of PASS should always be a forced re-lint. |

---

## Appendix — the two shapes worth naming

Every finding above collapses into one of two shapes, and both are cheap to test for in
future reviews:

**Shape A — the gate that scans text instead of behaviour.** `overlay_hygiene_gate`
(substring scan), the ADR-0065 "one place" test (re-performs the arithmetic), the
ADR-0034 golden (keyword presence). *Test for it by asking: what literal edit to
production code would turn this red?* If the answer is "a rename", the gate is textual.

**Shape B — the guarantee whose enforcement is a pipe, a comment, or a human.** The CI
anchors step (`| tail -1`), `SALT_TABLE`'s docstring, AD-14's `cargo tree`, AD-11's
"operator review", AD-19's fmt/clippy floor. *Test for it by asking: if this were false
tonight, what would go red before morning?* If the answer is "nobody would know", the
guarantee is prose.
