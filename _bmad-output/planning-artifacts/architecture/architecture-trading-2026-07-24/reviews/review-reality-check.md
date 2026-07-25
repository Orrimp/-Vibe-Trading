# Reviewer Gate — REALITY-CHECK lens

- **Artifact under review:** `_bmad-output/planning-artifacts/architecture.md` (spine, 2026-07-24)
- **Lens:** brownfield reality-check — committed claims vs the repo. Scope: what the sibling
  rubric lens did NOT verify (crate map, anchors count, gate scripts, stack-pin existence,
  register rows, ADR 0054/0079, ci.yml were already confirmed there) plus freshly edited
  passages.
- **Date:** 2026-07-25 (completing the interrupted 2026-07-24 run)
- **Method:** direct grep/read of the working tree at `main` (5582a74) + `git log`/`git show`
  for dating. ~10 probe batches; every verdict below cites file:line evidence.

## Verdict summary

**CONDITIONAL PASS.** 14 of 16 checked claims verify byte-for-byte against the repo —
including the two subtlest ones (overlay shadow-not-fold semantics; the XOR-doc vs
`wrapping_add`-impl divergence the spine itself flags as AD-18's standing instance).
Two claims FAIL:

1. **The R1 forward-coverage gap is described as open, but R1 SHIPPED 2026-06-30**
   (commit `2106f4a`, ADR-0077) — four spine passages are stale (AD-8 seam 1, AD-8
   Enforced-by, Capability Map SUGGEST residual, Deferred R1 row).
2. **AD-14(c)'s live-edge enumeration is incomplete** — the `live` feature also gates
   `dep:reflection`, a fourth sibling edge absent from the spine's "only
   `agent`/`audit`/`llm`" list and from the mermaid diagram.

Neither failure invalidates the invariant structure; both are fix-with-an-edit accuracy
defects (the first is exactly the "spec status lags code" failure mode this repo's own
memory warns about).

## Claim-by-claim table

Verdicts: **CONFIRMED** (repo matches claim) · **FAILED** (repo contradicts claim) ·
**PARTIAL** (claim mostly holds; a stated sub-part does not).

| # | Spine claim (passage) | Repo evidence | Verdict |
|---|---|---|---|
| 1a | AD-14(c): `ui`'s engine-side edges are the optional `live`-gated set `agent`/`audit`/`llm` ("`llm` solely for `tracing_init`") | `crates/ui/Cargo.toml:326-341` — `live = ["dep:agent", "dep:llm", "dep:audit", "dep:reflection", "dep:tokio", "dep:futures", "dep:async-stream", "dep:tokio-util", "dep:anyhow", "dep:tracing-subscriber", "dep:uuid"]`. `dep:agent`/`dep:llm`/`dep:audit` present as claimed; `llm` dep comment `:134-135` confirms "llm::tracing_init::install_global … Optional dep gated on `live`" (T-RED-D10). **But** `dep:reflection` (`:333`, declared `:137-144`) is a FOURTH `live`-gated sibling edge the spine's "only … : `agent` (runtime host), `audit` (query surface), and `llm` solely for `tracing_init`" enumeration and the mermaid (`ui -. live .->` only agent/audit/llm) both omit. Manifest justifies it via ADR-0031 as "structurally equivalent to the existing `ui → agent → reflection` transitive edge" (agent re-exports reflection) — mitigating, but the spine text claims exhaustiveness and is not exhaustive. | **PARTIAL / FAILED (enumeration)** |
| 1b | AD-14(c): `live` default-on for the operator bundle since 2026-05-25 | `crates/ui/Cargo.toml:238` — `default = ["live", "yahoo", "binance"]`; comment `:234` "2026-05-25: promoted from explicit-opt-in to default per operator request". | **CONFIRMED** |
| 1c | AD-14(c): no `strategy`/`exec`/`forecast` dependency in `ui`, any build or feature set | `crates/ui/Cargo.toml` `[dependencies]` (`:76-153`) — trading_core, data (opt), reports, backtest, rust_decimal_macros, rand, rand_chacha, libc, thiserror, serde, serde_json, tracing, rust_decimal, time, smol_str, clap, iced, agent (opt), llm (opt), audit (opt), reflection (opt), tokio/futures/async-stream/tokio-util/anyhow/tracing-subscriber/uuid (opt), windows (cfg). **Zero** direct `strategy`/`exec`/`forecast` edges in `[dependencies]`, `[dev-dependencies]`, or any feature stanza. | **CONFIRMED** |
| 1d | (mandate note) dev-deps content | `[dev-dependencies]` `:175-222`: criterion, insta, tokio(test-util), futures, **agent** (`:186`), data(fixtures), **audit** (`:196`), tempfile, iced_test, image-compare, image, proptest, base64, chrono. Engine crates `agent` + `audit` appear as non-optional dev-deps (test-only; both are already-sanctioned live edges as normal deps). No strategy/exec/forecast directly — though the `agent` dev-dep transitively pulls them into *test* builds, as it also does in any default (`live`) build; AD-14(c)'s letter ("lib + every bin") and its `cargo tree -p ui` **diff** gate (unchanged, not absent) are consistent with this. | note, no verdict |
| 2a | AD-8 seam 1(ii): `advisor_field()` concatenates pre-registered lists via extend, in `crates/ui/src/leaderboard/runner.rs` | `runner.rs:55-61` — `let mut field = backtest::BakeoffConfig::default_field(); field.extend(…default_ensemble_field()); field.extend(…default_macro_field());`. Append-only concatenation as claimed. | **CONFIRMED** |
| 2b | AD-8 seam 1(iii): `agent::runtime::build_registry_for` exists, matches on literal arm ids, unknown id ⇒ forward run fails | `crates/agent/src/runtime.rs:335` `pub fn build_registry_for(`; literal-string match arms `:359-624` (`"v0.sma"`, `"v0.5.macd"`, … `"v0.macro_riskon"`); fallthrough `:626-633` `unknown => anyhow::bail!("… unknown strategy id … refusing to silently fall back to SmaCrossover proxy (F5b anti-fake gate)")`. Failure *mode* as described. | **CONFIRMED** |
| 2c | AD-8 Enforced-by: "`forward_run_engine_fidelity.rs` covers **only the F5b-era ids**"; seam 1: "**Until refactor R1 lands (Deferred)**, forward-run support is per-arm … a crowned arm missing home (iii) fails the SUGGEST run"; Capability Map SUGGEST: "*Residual: the forward run covers the F5b-era arms — post-F5b arms are pending refactor R1*"; Deferred: "`build_registry_for` **lacks** forward-run coverage for the 14 arms added after F5b — crowning one fails the forward run" | **R1 shipped 2026-06-30**: commit `2106f4a` "feat(advisor-forward-fidelity-coverage): R1 — 14 post-F5b arms forward-buildable" (+149 lines runtime.rs, +254 lines fidelity test, ADR-0077 `spec/architecture/adr/0077-forward-fidelity-coverage.md`, `spec/v2/advisor-forward-fidelity-coverage/`). `CHANGELOG.md:76`: "closes the F5b forward-run coverage hole: **all 14 post-F5b arms** (combination / signal-library / DVOL / macro) are now forward-buildable (`build_registry_for`), so crowning any of them no longer `bail!`s the forward paper-run. ADR-0077." Test evidence: `crates/agent/tests/forward_run_engine_fidelity.rs` asserts must-build for `v0.donchian_break/:385`, `v0.donchian_floor/:406`, `v0.vol_breakout/:427`, `v0.roc_momentum/:448`, `v0.obv/:469`, six `v0.8.vote.*/:490-564`, `v0.dvol_regime/:575`, `v0.macro_riskon/:599` — i.e. far MORE than the F5b-era ids. Registry coverage is complete: all 19 advisor-field ids (`default_field` 10 @ `crates/backtest/src/bakeoff/mod.rs:562+`, `default_ensemble_field` 8, `default_macro_field` 1) + `v0.buyhold` each have a match arm in `runtime.rs:359-624`. | **FAILED (stale ×4 passages)** |
| 2d | AD-8 seam 1: "no completeness test exists today (the cheap closure, see Deferred)" — i.e. no mechanical `advisor_field()` ↔ `build_registry_for` iteration test | Grep of `crates/agent/tests/` + `crates/ui/tests/` for `advisor_field`: only hit is `crates/ui/tests/leaderboard_populated_render.rs:814` (arm-count render assert). The fidelity test enumerates arms **individually**; no test iterates the live field against the registry. This *half* of the Deferred row survives R1 — as a drift tripwire for FUTURE arms, no longer as cover for a live gap. | **CONFIRMED (still true)** |
| 3 | AD-8 seam 2: wrappers **shadow** (do not fold) the inner overlay's scale — `quantity_scale` returns own multiplier, never calls `self.inner.quantity_scale` | `crates/strategy/src/drawdown_control_overlay.rs:390-394` — `fn quantity_scale(…) { self.cached_multiplier.to_f64().unwrap_or(1.0) }`; `crates/strategy/src/vol_targeting_overlay.rs:735-737` — `fn quantity_scale(…) { self.scale_cache.get(symbol).copied().unwrap_or(1.0) }`. Grep for `inner.quantity_scale` across both files: **zero hits**. Stacking WOULD silently no-op the inner layer, exactly as the spine warns. | **CONFIRMED** |
| 4a | AD-17: frozen 16-entry salt table | `crates/backtest/src/bakeoff/bootstrap.rs:63` — `pub(crate) const SALT_TABLE: [u64; 16]`; doc `:60-62` "The table is frozen". | **CONFIRMED** |
| 4b | AD-18 standing instance: rustdoc says XOR, `derive_master_seed` ships `wrapping_add` | `bootstrap.rs:87-88` doc: "The **XOR** with the salt ensures different candidates draw different resample sequences…"; impl `:90-93`: `let salt = SALT_TABLE[candidate_index % SALT_TABLE.len()]; bakeoff_seed_u64.wrapping_add(salt)`. Divergence exists verbatim, exactly as the spine records it. | **CONFIRMED** |
| 4c | AD-17: "the ~20-arm advisor field already wraps the 16-salt table" | `runner.rs:63-74` — `advisor_field_arm_count()` = `advisor_field().len() + 1` = **20** for BTC/ETH (19 field ids + benchmark; 19 total for symbols where the DVOL arm is filtered). 19-20 candidates > 16 salts ⇒ `candidate_index % 16` wraps (indices ≥ 16 share salts), as claimed. | **CONFIRMED** |
| 5a | AD-10: root dep-opt dev profile is part of the latency contract | root `Cargo.toml:42-43` — `[profile.dev.package."*"]` / `opt-level = 3`. | **CONFIRMED** |
| 5b | AD-10: rendering is CPU tiny-skia, never wgpu | `crates/ui/Cargo.toml:115` — `iced = { version = "=0.14.0", default-features = false, features = ["tiny-skia", "thread-pool", "advanced", "canvas"] }`. `"tiny-skia"` present, `"wgpu"` absent, defaults off (exact `=0.14.0` pin also matches the Stack row). | **CONFIRMED** |
| 6a | AD-2: `spec/anchors.toml` exists (registry substrate) | `spec/anchors.toml` present (55,136 bytes). (119/119 row count verified by sibling lens.) | **CONFIRMED** |
| 6b | AD-19: LLM artifacts secret-scan script exists | `scripts/check_no_secrets_in_llm_artifacts.sh` present (5,445 bytes). | **CONFIRMED** |
| 6c | Structural Seed / operational envelope: `lab-runs/` and `plan-exports/` are git-ignored sibling roots | `.gitignore:6` `/lab-runs/`; `.gitignore:13` `/plan-exports/` (with the ADR-0055 precedent comment at `:10`). | **CONFIRMED** |
| 7 | Stack rows (late edits): clap 4.5.37 pin, thiserror 2.0, anyhow 1.0, time 0.3 | root `Cargo.toml:141` `clap = { version = "4.5.37", … }`; `:81` `thiserror = { version = "2.0" }`; `:82` `anyhow = { version = "1.0" }`; `:98` `time = { version = "0.3", … }`. All four exact. | **CONFIRMED** |
| 8 | Consistency Conventions / Structural Seed: audit schema evolves by appending numbered migrations | `crates/audit/migrations/` — `001_chart_of_accounts.sql` … `013_equity_snapshots.sql`, 13 contiguous `NNN_*.sql` files. Substrate for the append-only claim exists as described. | **CONFIRMED** |

## Failed claims — required spine edits

### F-1 (major, staleness): R1 forward-coverage is DONE — four passages describe a closed gap as open

The spine was migrated from a 2026-07-10 source and repo-verified for CI/P4/P5 status, but
the R1 status carried over stale from the ~2026-06-28 v2 scoping (the repo memory's
"spec status lags code" trap, in the spine itself). Ground truth: commit `2106f4a`
(2026-06-30) + ADR-0077 + `CHANGELOG.md:76` — all 14 post-F5b arms (signal-library ×5,
vote-combination ×6-of-8, DVOL, macro) are forward-buildable; the fidelity test asserts
must-build per arm; every id in today's 19-arm advisor field resolves in
`build_registry_for`.

Passages to fix (all four say the same wrong thing):

1. **AD-8 seam 1** — "Until refactor R1 lands (Deferred), forward-run support is per-arm,
   not automatic — a crowned arm missing home (iii) fails the SUGGEST run". → R1 landed
   (ADR-0077); the per-arm principle and the `bail!` failure mode remain true for *future*
   arms, but the "until … lands" framing and the Deferred pointer are stale.
2. **AD-8 Enforced-by** — "`forward_run_engine_fidelity.rs` covers only the F5b-era ids —
   the completeness gap is named in Deferred". → The test covers the full current field.
   The surviving gap is only the *mechanical* `advisor_field()` ↔ `build_registry_for`
   iteration test (see F-1-residual below).
3. **Capability Map SUGGEST row** — "*Residual: the forward run covers the F5b-era arms —
   post-F5b arms are pending refactor R1 (AD-8 seam 1, Deferred).*" → Delete or replace
   with "forward coverage is complete for the shipped field (ADR-0077)".
4. **Deferred table, R1 row** — "`build_registry_for` lacks forward-run coverage for the
   14 arms added after F5b — crowning one fails the forward run." → R1 is shipped; if a
   Deferred row survives at all, it is only the completeness-test half.

**F-1-residual (true and worth keeping):** no test iterates the live `advisor_field()`
against `build_registry_for` (only per-arm asserts + the `unknown => bail!` arm). As a
tripwire for arms added *after* today, the "cheap closure" completeness test remains a
legitimate Deferred item — reworded as future-drift protection, not as a live gap.
ADR-0077 should also join the AD-8 / Capability-Map ADR citations.

### F-2 (moderate, completeness): AD-14(c) omits the `live`-gated `ui → reflection` edge

`crates/ui/Cargo.toml:333` (`"dep:reflection"` in the `live` stanza; dep declared
`:137-144`). The spine's rule (c) reads "`ui`'s **only** engine-side edges are the
optional, `live`-feature-gated bootstrap set …: `agent` (runtime host), `audit` (query
surface), and `llm` solely for `tracing_init`" — and the mermaid draws exactly three
dotted `live` edges. The manifest's own rationale (ui-rethink-phase-d T-D-N1): the edge is
ADR-0031-sanctioned as "structurally equivalent to the existing `ui → agent → reflection`
transitive edge (agent already re-exports reflection)", used by the trail-mirror
Subscription bridge (`TrailMirrorHandle`/`TrailMirrorTick`). Not a *violation* of any
forbidden edge (only `strategy`/`exec`/`forecast` are crossed out) — but the spine claims
exhaustiveness and is off by one. Fix: add `reflection` (trail-mirror bridge, ADR-0031) to
the enumeration + a fourth dotted mermaid edge, or soften "only" to name the three
*bootstrap* edges and cite ADR-0031 for reflection.

## Minor notes (accurate but worth recording)

1. **`ui` dev-deps carry engine crates `agent` + `audit`** (`crates/ui/Cargo.toml:186,196`,
   test-only). AD-14(c)'s letter binds "lib + every bin", so this is compliant, and both
   crates are already sanctioned `live` edges — but anyone tightening AD-14 enforcement
   into an automated lint (the Deferred dependency-edge lint) must decide explicitly
   whether dev-deps are in scope, or the lint will "discover" these.
2. **`cargo tree -p ui` on a default build contains `strategy`/`exec` transitively**
   (via the default-on `live` → `agent`). AD-14(c)'s gate is correctly worded as a *diff*
   gate ("unchanged"), not an absence gate — keep it that way; an absence assertion would
   be false today.
3. **`advisor_field_arm_count` is symbol-dependent** — 20 (BTC/ETH) vs 19 (DVOL arm
   filtered), `runner.rs:63-74`. The spine's "~20-arm" hedge is exactly right; a future
   editor should not "precisify" it to a single number.
4. **The `unknown => bail!` arm** (`runtime.rs:626-633`) is the enforcement teeth behind
   AD-8's "explicit forward-ineligible registration" language — worth citing in AD-8 as
   the anti-silent-fallback gate (F5b anti-fake), since it is what makes a missing home
   (iii) *loud* instead of wrong.
5. **`v0.8.vote.majority`/`v0.8.vote.unanimous`** were forward-buildable pre-R1
   (`runtime.rs:463`, grouped with the F5b-era arms) — consistent with "14" (not 16)
   post-F5b arms in the R1 accounting; the ensemble field is 8 ids, 6 of which were R1
   additions.
6. **iced pin is exact** (`=0.14.0`, `crates/ui/Cargo.toml:115`) — stricter than the
   Stack table's bare "0.14.0" implies; the Stack row is accurate as written.

## Scope not re-verified (sibling rubric lens)

17/17 crate map · 119 anchor rows · gate scripts + named tests existence · stack pins
iced/tokio/rust_decimal/polars/rand/candle/proptest/insta/trybuild/criterion/reqwest/rustc ·
tract absence · do-not-build rows B-2/E-1 · ADR 0054/0079 facts · ci.yml active.
