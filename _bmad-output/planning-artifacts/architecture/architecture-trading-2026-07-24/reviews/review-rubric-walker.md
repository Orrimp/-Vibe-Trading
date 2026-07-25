# Rubric Walker Review — architecture spine `trading`

- **Artifact:** `_bmad-output/planning-artifacts/architecture.md` (brownfield spine, all ADs [ADOPTED])
- **Lens:** RUBRIC WALKER (good-spine checklist, item by item)
- **Date:** 2026-07-24
- **Scope note:** mechanical lint (placeholders, AD monotonicity, Binds/Prevents/Rule presence, Stack pins) already passed and is not re-checked here.

## Gate verdict

**CONDITIONAL PASS — one critical ratification defect in AD-14(c) must be fixed before this spine can be trusted as law; everything else is fixable in place.** The spine is otherwise an unusually faithful, well-enforced compression of the brownfield reality: 17/17 crates verified, 119/119 anchors real, every cited gate script and named test exists on disk, CI posture and P4/P5 ship-status corrections are accurate.

---

## Checklist walk (verdict per item)

| # | Checklist item | Verdict |
|---|---|---|
| 1 | Fixes the real divergence points for the level below; misses none | **MOSTLY** — the 15 classic axes (gate freeze, anchors, dep law, PIT, money, UI pixels, seams, register, lifecycle triad, release gates, determinism, ADR registry, paper-only, overlay e2e, platform/CI) are all present; three real axes are un-lifted (findings C-1 consequence, H-1, M-1, M-2) |
| 2 | Every AD's Rule is enforceable and prevents its divergence | **MOSTLY** — 18 of 19 ADs name a live, verified enforcement artifact; AD-14 alone has no `Enforced by:` clause and its "hard gate" is manual (H-2) |
| 3 | Nothing under Deferred could let two units diverge | **ONE EXCEPTION** — R1 forward-coverage is divergence-capable as deferred (H-1); the other nine rows are safely fenced |
| 4 | Named tech verified-current for this repo | **PASS** — spot-checked against `Cargo.toml`/`Cargo.lock`/toolchain; one version-string imprecision (L-2) |
| 5 | Ratifies rather than contradicts the brownfield codebase | **FAIL on one law** — AD-14(c)'s "never depends on `llm`" is falsified by the default build (C-1); everything else ratifies cleanly |
| 6 | Covers the driving journey (DATA→CALIBRATE→ANALYZE→SUGGEST) | **PASS** — all four stages mapped to screen/crates/ADRs incl. stepper projection; SUGGEST carries an undisclosed residual (part of H-1) |
| 7 | Every initiative-owned dimension decided/deferred/open — esp. operational envelope | **PASS** — the Operational envelope paragraph decides deployment (local single-operator desktop), environments (none beyond the operator's machine), infra (no cloud; GitHub Actions CI), distribution (cargo build; packaging explicitly Deferred), durable state, secrets. No wholly-silent dimension; two sub-axes are decided only in passing (M-1, M-2) |
| 8 | Terse and convergent | **PASS** — decisions-not-rationale mostly held; one ADR-content duplication (L-3) |

---

## Findings

### CRITICAL

#### C-1 — AD-14(c) states a dependency law the default build violates (`ui` → `llm`)

- **Location:** AD-14, rule (c) ("`ui` (lib + every bin) never depends on `strategy`, `exec`, `forecast`, or `llm`") + the mermaid `ui --x llm` edge.
- **Evidence (repo-verified):**
  - `crates/ui/Cargo.toml` declares `llm = { path = "../llm", optional = true }`, activated by the `live` feature, and `live` is in `default = ["live", "yahoo", "binance"]` — so plain `cargo build -p ui` resolves a **direct** `ui → llm` edge.
  - `crates/ui/src/bin/cockpit_live.rs:235` directly imports it: `llm::tracing_init::install_global(...)` — a ui **bin** depending on `llm`, exactly what the sentence forbids.
  - The ADR record itself still asserts the opposite as recently as P5: ADR-0023 ("Direct UI imports of `strategy`/`exec`/`models`/`llm` are reject-on-review") and ADR-0088's registry row ("no strategy/exec/models/llm edge, `cargo tree -p ui` unchanged").
- **Why critical:** this is the spine's most-cited structural law, and it is falsifiable-false against `Cargo.toml` today. The source doc (`spec/architecture/00-current-state.md` § dependency-direction invariants) carries the same flat claim, so the spine migrated a latent contradiction into the artifact that becomes authoritative at Phase 5b. A future story-builder either (a) flags every ui build as a violation, (b) "fixes" the repo by ripping `llm` out of the live bundle (breaking cockpit_live's tracing bootstrap), or (c) learns the spine's laws are approximate — all three are the divergence the AD exists to prevent. Note the *spirit* survives (no `strategy`/`exec`/`forecast` edge exists in `crates/ui/Cargo.toml`; `llm` use is confined to the tracing bootstrap, not `LlmProvider`), which is precisely why the fix is wording, not code.
- **Fix (concrete):** rewrite AD-14(c) to the enforceable truth, and route the carve-out to an ADR per AD-18:
  1. Law: "`ui` never depends on `strategy`, `exec`, or `forecast` (lib or any bin). `ui`'s engine access is the `live` feature's sanctioned bootstrap set `{agent, llm, audit}` (T904 live-cockpit-unified); within it, `llm` is restricted to `tracing_init` bootstrap in `cockpit_live` — no `LlmProvider` use in `ui`. `cargo tree -p ui` **unchanged** remains the per-change gate."
  2. Update the mermaid: drop `ui --x llm`, add `ui --> agent` (see L-1) and, if the restriction is kept, a footnote for the `llm` tracing-bootstrap edge.
  3. Because no ADR currently sanctions `dep:llm` in `live` (ADR-0023/0088 say the opposite), either log an open question / follow-up ADR ("legitimize or eliminate the `live` `dep:llm` edge") or record it as a known deviation — silence is the one option AD-18 forbids.

### HIGH

#### H-1 — Deferred row "R1 forward-coverage refactor" is divergence-capable as written

- **Location:** § Deferred, row 7; also AD-8's Rule and the Capability Map SUGGEST row.
- **Evidence:** `build_registry_for` (crates/agent/src/{runtime,config,plan}.rs) has no forward-run coverage for the 14 arms added after F5b — crowning one fails the forward run (per the v2-architect code-grounded finding the row itself cites). Meanwhile AD-8 actively invites new arms ("new arms only ever mean more candidates face the same bar") and AD-16 gates only overlays/sizing modifiers, not arm forward-coverage.
- **Why high:** checklist item 3's exact failure mode. Two independent future arm-builders can each land a gate-legal arm (seam 1) that ranks on ANALYZE but crashes SUGGEST when crowned; nothing in any AD prevents it, and the gap compounds silently with every new arm. The Deferred row's "why it can wait" column ("scoped as v2 refactor R1") is a status, not a guard. The Capability Map's SUGGEST row also presents forward paper-trade as unconditional, hiding the residual from journey consumers.
- **Fix (concrete):** (1) add one sentence to AD-8's Rule: "a new **arm** additionally ships `build_registry_for` forward-run coverage (registry entry + test), or records its exclusion — until refactor R1 lands, forward-run support is per-arm, not automatic"; (2) rewrite the Deferred row's second column as a real guard/revisit condition ("safe to defer only while no new arm lands uncovered; any new arm triggers the guard above; R1 closes it wholesale"); (3) add a one-clause caveat to the SUGGEST row ("forward run covers the F5b-era arms; post-F5b arms pending R1").

#### H-2 — AD-14 is the only AD with no `Enforced by:` clause, and its strongest instrument is not automated

- **Location:** AD-14 (contrast every other AD, which closes with `**Enforced by:** <artifact>`).
- **Evidence:** "**`cargo tree -p ui` unchanged is a hard gate** on advisor work" — no hit for `cargo tree` in `scripts/`, `.github/`, or `crates/ui/tests/`; the check is review-time manual. Rules (a)/(b)/(d)/(e) are likewise enforced only by review convention (no cargo-deny bans, no metadata lint asserting the forbidden edges).
- **Why high:** checklist item 2 asks whether each Rule is enforceable as stated. The structural law — the AD with the widest Binds ("all 17 crates; every new Cargo.toml edge") — is the one whose enforcement is (i) undeclared and (ii) overstated ("hard gate" for a manual diff). This is also the AD where the one real drift already happened (C-1), which is empirical evidence review-only enforcement leaks.
- **Fix (concrete):** add the missing clause honestly: "**Enforced by:** review against this AD + a manual `cargo tree -p ui` diff on advisor changes (not automated)." Then add one row to § Deferred: "Dependency-edge lint (cargo-metadata assert of AD-14 forbidden edges, incl. the `cargo tree -p ui` snapshot) — small script; revisit on the next AD-14 near-miss." That keeps the spine truthful today and names the cheap closure.

### MEDIUM

#### M-1 — The renderer/build-profile contract (tiny-skia over wgpu; dev dep-opt) is un-ratified, yet AD-10 depends on it

- **Location:** absent — belongs in AD-10 or § Consistency Conventions; today it lives only in root `Cargo.toml` comments.
- **Evidence:** root `Cargo.toml`: the cockpit renders via CPU `tiny-skia` "chosen for snapshot-test determinism over GPU `wgpu`", plus `[profile.dev.package."*"] opt-level = 3` fixing a measured 707 ms → 17 ms per-frame render (the operator's interaction-latency contract); `crates/ui/Cargo.toml` pins `iced` features `["tiny-skia", ...]`.
- **Why medium:** a future ui story that enables `wgpu` (the iced-idiomatic default) or "cleans up" the profile block silently breaks AD-10's `Emulator::screenshot` determinism and the operator latency contract — a real divergence axis for the level below, guarded today only by comments. The spine's Stack table names the vendored `iced_tiny_skia` fork but never states the renderer decision itself.
- **Fix:** one Conventions row (or an AD-10 Rule sentence): "Rendering: CPU `tiny-skia` (never `wgpu`) — snapshot determinism depends on it; the root-`Cargo.toml` dep-opt dev profile is part of the operator latency contract (`spec/v1/cockpit-performance-and-input-responsiveness/`)."

#### M-2 — Audit-ledger migration discipline is decided only in passing

- **Location:** § Structural Seed, Operational envelope ("the audit SQLite ledger (additive numbered migrations)").
- **Evidence:** `crates/audit/migrations/001…008_*.sql` exist; the additive/numbered discipline is real but appears in the spine as a parenthetical with no rule and no enforcement pointer.
- **Why medium:** shipped-schema edits are a classic two-builder divergence (editing `005_*.sql` in place vs. appending `009_*.sql` corrupts every existing operator ledger); the initiative altitude owns this operations sub-axis and currently decides it only implicitly.
- **Fix:** one Conventions row under "Data & formats" or "Errors, logging, state": "Audit schema evolves by appending numbered migrations only — a shipped `NNN_*.sql` is immutable (same immutability class as anchors)." Cite the audit section file / governing ADR.

### LOW

#### L-1 — Mermaid omits the sanctioned `ui --> agent` edge the AD text names

- **Location:** AD-14 diagram vs. AD-14(c) text ("through the sanctioned `backtest`/`agent` channel seams").
- **Evidence:** `crates/ui/Cargo.toml` `live` feature: `dep:agent`; `ui/src/live.rs` consumes `agent::`; the diagram draws `ui --> backtest` but not `ui --> agent`. Undrawn edges are declared unconstrained, so this is not wrong — but a load-bearing sanctioned seam named in the law's own text deserves drawing; its absence invites a false "new edge" alarm on the very gate AD-14 prescribes.
- **Fix:** add `ui --> agent` (annotated `live`-feature) to the diagram.

#### L-2 — Stack: clap row breaks the table's own "workspace-manifest pins shown" promise

- **Location:** § Stack, clap row ("4.x (lock 4.6.1)").
- **Evidence:** workspace pin is `clap = { version = "4.5.37" }` (`Cargo.toml`); lock 4.6.1 is correct.
- **Fix:** "4.5.37 pin (lock 4.6.1)". (Everything else spot-checked true: iced `=0.14.0`, tokio 1.44/1.52.3, rust_decimal 1.36/1.42.0, polars 0.46, rand 0.9, candle-core 0.9.2, proptest/insta/trybuild 1.6/1.42/1.0, criterion 0.5, reqwest 0.12 rustls, rustc 1.94.1, `tract` absent from `Cargo.lock` — the spine's tract correction is right.)

#### L-3 — Crown-credibility paragraph duplicates ADR-0085 content (terseness)

- **Location:** § Capability → Architecture Map, the "Crown-credibility states (ADR-0085)" paragraph.
- **Evidence:** the three-state semantics *and* the rationale ("the DSR is on the max-Sharpe active *loser*, so a badge would mislead") restate ADR-0085/the source's P1 section nearly verbatim; the ANALYZE row plus AD-12 already carry the decision.
- **Why low:** checklist item 8 — rationale belongs in the ADR; the spine needs only the decision and the pointer.
- **Fix:** compress to one sentence ("Banner co-presents `crown_credibility(outcome, scorecard)`: `Passes` ✓ / `WeakEvidence` ⚠ WARN / `NotApplicable` no badge — semantics + rationale in ADR-0085") and delete the rest.

---

## What was verified clean (no finding)

- **Crate map:** 17/17 members match root `Cargo.toml` exactly; no `crates/models`; `forecast`+`features` claim holds.
- **Anchors:** `spec/anchors.toml` contains exactly 119 SHA rows; `scripts/verify_anchors.sh` exists; dual-anchor System 2 (`check_determinism_anchors.py`, AD-17) exists.
- **AD-13 correction is right:** `.github/workflows/ci.yml` is live (not `.deferred`) — the spine correctly overrides the stale 2026-07-10 source, with the restatement flagged in place.
- **Enforcement artifacts all exist:** `scorecard_does_not_change_ranking` / `turnover_does_not_change_ranking` (bakeoff), `venue_filter_default_is_none` / `paper_step_none_is_byte_identical`, `BenchmarkWins`-reachability (`robustness_bootstrap_bites.rs` invariant 3), `narration_faithfulness.rs`, `vol_targeting_overlay_end_to_end.rs`, `determinism.rs` + `multi_pair_determinism.rs`, `core/src/pit.rs`, `check_no_raw_asof_join.sh`, `spec_lint.py` (both AD-4 rules per the migration plan's inventory), `adr_registry_check.py`, `check_no_secrets_in_llm_artifacts.sh`.
- **Seams are real:** `default_field()` / `default_ensemble_field()` (`bakeoff/mod.rs:562/641`), `Strategy::quantity_scale` (overlay impls).
- **Register claims:** `do-not-build-register.md` rows B-2 (paper-only) and E-1 (DSR veto) match AD-15/AD-12 verbatim in substance; ADR-0054 "intentionally skipped" and 0079 registry-row-only both check out.
- **Migration posture (checklist 1, migration consumers):** the three phase-critical divergence points are each pinned in place — AD-2 (Phase 3 layout-preserving `git mv`, SHAs survive), AD-4 (Phase 5b triad re-founding), header + Decision Record (Phase 4 ADR-link rebase) — consistent with `spec/dev-notes/bmad-migration-plan-2026-07-24.md`.
- **Deferred rows other than R1:** each is fenced by a standing decision (ADR-0080/0057/0060/0075/0067/0065/0007) or explicitly unowned (packaging); none lets two units diverge.

## Bottom line

Fix C-1 (reword AD-14(c) to the enforceable truth + route the `live`/`llm` carve-out to an ADR), close H-1 with the one-sentence arm-coverage guard, and add AD-14's honest `Enforced by:` line (H-2). With those three edits this spine is a trustworthy law-book for future feature/story builders and the migration phases; the medium/low items are single-row polish.
