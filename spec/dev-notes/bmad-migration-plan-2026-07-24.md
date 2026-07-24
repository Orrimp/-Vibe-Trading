---
slug: bmad-migration-plan
status: proposed
owner: architect
updated: 2026-07-24
---

# Full Migration to BMAD-METHOD v6.10.0 — Ratification-Ready Plan

> **DESIGN ONLY.** This document is the buildable migration plan the operator ratifies
> *before* any file moves or installs. Nothing in the real repo has been installed, moved,
> or edited by this pass — this plan is the single new file. It is deliberately the **last
> spec-native document**: it plans `spec/`'s own retirement. Once Phase 4 runs, this file
> itself moves to `docs/dev-notes/`.
>
> Grounded against the reference install at
> `…/scratchpad/bmad-probe/` (BMAD v6.10.0, installed 2026-07-24) — skill files, templates,
> `config.yaml`, and manifests were read, not guessed. Machinery touchers are grep-grounded
> against `scripts/`, `crates/`, `.github/`, `spec/anchors.toml`, and `spec/trace.toml`.

---

## 0. TL;DR + recommendation

**Verdict: migrate, in 7 additive-then-cutover phases, gates green at every commit.** BMAD v6.10.0
is a clean superset of our *planning* workflow (PRD → architecture → epics/stories → sprint-status →
story-driven dev/review) and installs with **zero collisions** against our tooling. But BMAD has **no
equivalent** for five pieces of our machinery — byte-SHA anchors, `trace.toml`, the re-founded
`spec_lint` triad, the ADR registry, and the `evidence/*/reports/` corpus. Full migration **ports**
these (re-homes + repoints), it does **not** delete a single guarantee. The heart of the work is (a)
transforming ~155 `feature.md` folders into **1:1 stories under 7 epics** and (b) a **base-swap** of the
anchor/report resolvers from `spec/` to two new roots (`evidence/` for the immutable corpus, `docs/` for
project knowledge). The June-2026 spec-reorg playbook scales up: **each phase transforms content and
repoints its machinery in the same commit**, with `verify_anchors` 119/119 and a working `spec_lint`
before *and* after.

---

## 1. BMAD v6.10.0 shape (verified from the sandbox)

| Concept | Where BMAD puts it | Source of truth |
|---|---|---|
| Persona agents | `.claude/skills/bmad-agent-{analyst,architect,dev,pm,tech-writer,ux-designer}` | skill dirs + `customize.toml` |
| Workflows (~47 skills) | `.claude/skills/bmad-*` (prd, create-architecture, create-epics-and-stories, sprint-planning, create-story, dev-story, code-review, document-project, retrospective, correct-course, customize, shard-doc…) | skill-manifest.csv |
| Module + core config | `_bmad/bmm/config.yaml`, `_bmad/core/config.yaml` | installer-generated |
| Manifests | `_bmad/_config/{manifest.yaml,skill-manifest.csv,files-manifest.csv}` | installer |
| **Customization mechanism** | `_bmad/custom/bmad-agent-<role>.toml` (team, committed) + `.user.toml` (gitignored) | resolved by `_bmad/scripts/resolve_customization.py` |
| Planning artifacts | `_bmad-output/planning-artifacts/` (PRD.md, architecture.md, `*epic*.md`) | `bmm/config.yaml: planning_artifacts` |
| Implementation artifacts | `_bmad-output/implementation-artifacts/` (`sprint-status.yaml` + story files) | `bmm/config.yaml: implementation_artifacts` |
| Project knowledge (brownfield) | `docs/` (project-overview, source-tree, deep-dives, `project-scan-report.json`) | `bmm/config.yaml: project_knowledge` |
| AI implementation rules | `project-context.md` under `output_folder` (`_bmad-output/`) | `bmad-generate-project-context` |

**Load-bearing facts:**
- **Config keys** (`_bmad/bmm/config.yaml`): `planning_artifacts`, `implementation_artifacts`,
  `project_knowledge: "{project-root}/docs"`, `output_folder: _bmad-output`. Everything is a
  `{project-root}`-relative token — the paths are *config*, so re-homing is a config edit, not a fork.
- **Every agent auto-loads project knowledge**: the default
  `persistent_facts = ["file:{project-root}/**/project-context.md"]` in each `bmad-agent-*/customize.toml`
  means whatever lands in `project-context.md` is context for *every* persona on activation. This is the
  primary seam for our non-negotiables.
- **Customization is sparse TOML overrides** in `_bmad/custom/`: append `persistent_facts`/`principles`/
  `activation_steps_*`, override scalars (`role`, `communication_style`, `icon`, `*_template`), merge menu
  items by `code`. Base → team → user merge via `resolve_customization.py`. This is the **official** home
  for the six agents' project knowledge.
- **Story status vocabulary** (`sprint-status.yaml`): epics `backlog|in-progress|done`; stories
  `backlog|ready-for-dev|in-progress|review|done`; retros `optional|done`; action-items `open|in-progress|done`.
  Our ~155 shipped features map cleanly to `done`.
- **gitignore posture**: only `_bmad/custom/*.user.toml` is ignored. `_bmad-output/`, `docs/`, and the
  team `.toml` overrides are **committed** — our migrated content stays in git.

---

## 2. Target tree (one screenful)

```
trading/
├── AGENT.md               # REWRITTEN around the BMAD cycle; non-negotiables VERBATIM
├── CLAUDE.md              # REWRITTEN; file-precedence repointed; non-negotiables FROZEN
├── CHANGELOG.md           # UNCHANGED, stays at root — the "done" index + triad anchor
├── README.md              # repointed front-door links
├── Cargo.toml · crates/ · src/ · research/ · vendor/   # code unchanged except 19 path-repoint files
├── evidence/              # NEW ── byte-immutable report corpus (was spec/**/reports/)
│   ├── v1/<slug>/reports/…      # git mv, layout preserved → resolver base-swap only
│   ├── v2/<slug>/reports/… · v3/<slug>/reports/…
│   └── v5-latency-slippage-sim*/reports/…
├── docs/                  # NEW ── BMAD project_knowledge
│   ├── dev-notes/ · runbooks/ (+artifacts/) · design/ · ui-design-principles.md
│   ├── do-not-build-register.md · research-gap-analysis.md
│   └── project-scan-report.json          # bmad-document-project output
├── _bmad/                 # BMAD install (config, manifests, scripts)
│   └── custom/            # ── the six agents' project knowledge lives here
│       ├── bmad-agent-{analyst,architect,dev,pm,ux-designer}.toml
│       └── bmad-{code-review,prd,create-epics-and-stories,dev-story}.toml   # workflow overrides
├── _bmad-output/
│   ├── project-context.md                # NON-NEGOTIABLES + AI rules (every agent loads this)
│   ├── planning-artifacts/
│   │   ├── PRD.md                         # was spec/product.md
│   │   ├── architecture.md               # was spec/architecture/00–12*.md (spine)
│   │   ├── architecture/decisions/       # was spec/architecture/adr/ (86 ADRs + README registry)
│   │   ├── epics/epic-0{1..7}-*.md        # 7 epics = shipped tranches
│   │   ├── trace.toml                    # was spec/trace.toml (ported ledger, repointed)
│   │   └── backlog.md                    # was spec/backlog.md (forward queue only)
│   └── implementation-artifacts/
│       ├── sprint-status.yaml            # generated; ~120 top-level stories, ~all done
│       └── stories/{epic}-{story}-{slug}.md
└── .claude/
    ├── agents/            # RETIRED (content → _bmad/custom/) — deleted in Phase 5c
    └── skills/            # KEEP our 14 harness skills + ADD ~47 bmad-* (zero collision)
```

`spec/` is **gone** after Phase 5. `lab-runs/` (ADR-0055 sibling root) is untouched — `evidence/`
joins it as a second sibling root, which the existing sibling-root model in
`crates/ui/src/lab/equity_loader.rs` already anticipates.

---

## 3. Install into the real repo — collision analysis

**Command (unchanged, non-interactive):**
```
npx bmad-method@6.10.0 install --directory . --modules bmm --tools claude-code -y
```

**Filesystem effect is purely additive.** The installer writes only under `.claude/skills/bmad-*`,
`_bmad/`, `_bmad-output/` (empty), and `docs/` (empty). It does **not** overwrite `CLAUDE.md`,
`AGENT.md`, `README.md`, our `.claude/agents/*`, or our non-`bmad-*` skills.

**Skill-name collisions: ZERO.** Grep-confirmed — our 14 skills are `backtest, capture-screenshot,
cockpit-smoke, present-results, rust-{bench,build,coverage,mutants,test,validate}, spec-{brief,lint,update},
verify-anchors`. None start with `bmad-`; BMAD's ~47 all do. **Keep all 14** — they are harness/CI tooling
(cargo gates, anchor verify, backtest, screenshot capture, spec lints) that BMAD lacks. Two of ours
(`spec-brief`, `spec-lint`) are re-founded on the new layout in Phase 5; `spec-update` is superseded by
the BMAD write-path but kept as a thin shim (see §10 open decision).

**Agent collisions: ZERO.** BMAD personas are *skills* (`.claude/skills/bmad-agent-*`), not
`.claude/agents/*`. Our 9 `.claude/agents/*.md` are untouched by install and retire in Phase 5c after
their knowledge is folded into `_bmad/custom/`.

**Config the installer generates** must be hand-tuned once (Phase 0): set `user_name`, `project_name:
trading`, and leave `planning_artifacts`/`implementation_artifacts`/`project_knowledge` at defaults (our
target tree adopts them verbatim).

---

## 4. Content mapping (the heart)

### 4.1 Mapping table

| `spec/` source | BMAD-native target | Transform | Machinery to repoint |
|---|---|---|---|
| `product.md` (57 KB) | `planning-artifacts/PRD.md` | Reshape to the v6 PRD spine (Vision · JTBD/UJ · Glossary · Features w/ nested `FR-N` · Non-Goals · MVP Scope · Success Metrics + **counter-metrics** · Open Qs · Assumptions Index) + Adapt-In clusters: *developer-product* (Rust runtime targets, perf budgets, dep policy) and *constraints/guardrails* (PAPER-ONLY, `Decimal` money, no-live-trading). Run `bmad-prd` in **Update** mode over `product.md`. | `queue_staleness_check.py` (backlog), none direct |
| `architecture.md` + `architecture/00–12*.md` (13 docs) | `planning-artifacts/architecture.md` | Merge the 13 section docs into the BMAD architecture **spine** (the cross-unit invariants). Run `bmad-create-architecture`/`bmad-architecture`. | doc-comment link sweep |
| `architecture/adr/` (86 ADRs + `README.md` registry) | `planning-artifacts/architecture/decisions/` | `git mv` — preserve `NNNN-*.md` numbering + the `## Registry` table verbatim. BMAD has no ADR concept; this is a **ported annex**. | `adr_registry_check.py` (path regex ×3) |
| ~155 `**/feature.md` | `implementation-artifacts/stories/{epic}-{story}-{slug}.md` (`Status: done`) + `epics/epic-0N-*.md` | Generate **1 story per feature** (see 4.2); frontmatter `status: shipped`→ story `done`; group under 7 epics. `retired` features → `done` with a `shipped_disposition: retired` note. | `spec_lint` re-founding, `spec_brief.py` |
| `backlog.md` | `implementation-artifacts/sprint-status.yaml` (live board) + `planning-artifacts/backlog.md` (forward queue) | Backlog **Queue** rows → `sprint-status.yaml` `backlog` entries; keep the prose backlog as forward-only. | `queue_staleness_check.py` |
| `trace.toml` (147 `[[req]]`) | `planning-artifacts/trace.toml` | Repoint path strings: `product`→`PRD.md`, `arch`→`architecture.md`/`decisions/`, `feature`→story path. `anchors` values are **names** — unchanged. **Ported ledger — no BMAD equivalent.** | `spec_lint` trace checks |
| `dev-notes/` (~39) | `docs/dev-notes/` | `git mv` | `operator_ledger_check.py`, `strings.rs`, ui-debugger ref |
| `runbooks/` (+`artifacts/`) | `docs/runbooks/` | `git mv` (incl. `artifacts/passive-baseline-2026-06-08/`) | `strings.rs` `KILL_RUNBOOK_LINK_PATH`, `baseline/loader.rs` |
| do-not-build register, research gap analysis | `docs/` + key rules folded into `project-context.md` | move file; **lift the "check the do-not-build register" non-negotiable into `project-context.md`** | none |
| `design/` (Lumen), `ui-design-principles.md` | `docs/design/`, `docs/ui-design-principles.md` | `git mv` | ui-designer doc refs |
| `**/reports/` + `v1..v3/**/reports/` (130 dirs, **119 anchors**) | `evidence/**/reports/` (layout preserved) | `git mv` — bodies are byte-immutable; **content-SHA survives rename**. Base-swap resolvers `spec/` → `evidence/`. | `verify_anchors.sh`, `spec_lint`, 19 code files, 5 scripts |
| `CHANGELOG.md` (root) | **unchanged** | none — already repo-root, not under `spec/` | `spec_lint` changelog check (re-based, path constant only) |

### 4.2 Epics / stories granularity — **recommendation: 1:1 story-per-feature under 7 epics**

**Recommended (durable choice).** One story per `feature.md`, `Status: done` derived from frontmatter,
grouped into **7 epics = the shipped tranches**:

| Epic | Tranche | ~count |
|---|---|---|
| epic-01 | Strategy & backtest engine (v0→v5 ladder + latency/slippage sim) | ~18 |
| epic-02 | Cockpit & UI (iced) — lab, live view, chart, quality gates, harnesses | ~42 |
| epic-03 | Advisor MVP (F1–F9 + EUR-FX + dynamic data + PIT discipline) | ~20 |
| epic-04 | v2 research-driven tranche (scorecard, turnover/tail, confidence, forward-coverage, vol/drawdown overlays, cost-opt-in, narration, no-alpha-CI, data-quality) | 11 |
| epic-05 | v3 "prove it's done" close-out (calibrate stepper, corpus-expansion, crown-credibility, handoff-export, lot-realism, pit-discipline) | 6 |
| epic-06 | Remediation P0–P8 + infra/tooling lints (adr-registry, operator-ledger, queue-staleness, determinism) | ~20 |
| epic-07 | Retired research lines (v2.5 DL forecaster programme; v3 vol/regime/xgboost) — `done` + `retired` | ~15 |

**Iteration folders fold as sub-tasks, not top-level stories.** The `-v0.2.0`/`-v0.3.0`, `-followup`,
`-noop-fix`, `-rebaseline` folders already roll up in `CHANGELOG_ROLLUP_ALLOWLIST` (in `spec_lint.py`).
**Reuse that allowlist as the fold map**: each becomes a `Tasks/Subtasks` bullet under its base-feature
story. This trims ~155 folders to **~120 top-level stories** + sub-tasks — matching the existing,
operator-blessed rollup convention.

**Grounds for 1:1 over a thin index:**
1. **Trace bijection.** `trace.toml` has 147 `[[req]]` rows ≈ 1 per feature. 1:1 keeps story ↔ req
   bijective, which is exactly what the re-founded ADR-0082 triad lint needs to check.
2. **Provenance.** Per-feature git history (`git log -- spec/<slug>/`) is the narrative record; 1:1
   stories preserve the mapping so `git log --follow` still tells each feature's story.
3. **Cost is trivial.** `sprint-status.yaml` handling ~120 `done` rows is a non-issue; BMAD's own
   template shows the exact map shape.

A thin index (1 story/epic, features as bullets) would **lose** the trace bijection and the per-feature
status gate — rejected.

**Story body for shipped features** uses `bmad-create-story/template.md` with `Status: done`,
back-filled `Acceptance Criteria` from the feature's `## Verification`/`Design` sections, `Dev Notes →
References` citing the `evidence/<slug>/reports/` anchors, and `Dev Agent Record` left as a historical
stub. These are **retro-generated**, so a lean back-fill (AC + references + anchor list) is sufficient —
we are not re-litigating shipped work.

---

## 5. Machinery re-engineering — full toucher list (grep-grounded)

Headline counts: **13 scripts**, **19 load-bearing code files** (31 literal `spec/` path refs), **1 CI
comment** (no functional CI path), **9 `anchors.toml` comment lines** (no functional key), plus **~470
rustdoc-comment refs** across `crates/` that are non-load-bearing (editable, not anchored) and get a
separate doc-hygiene sweep.

### 5.1 Scripts (13) — the anchor/lint/trace machinery

| Script | What breaks | Fix |
|---|---|---|
| `verify_anchors.sh` | base `$root/spec`; `migration_dir_v02..v05`; `canonical_dirs_pattern`; hardcoded namespace dirs `mc_reports_dir`, `mc_sweep_dir`, `mc_mr_dir`, `mc_carry_dir`, `mc_ts_dir`, `mc_horizon_dir`, `mc_basis_dir`, `mn_spread_dir` (all `spec/v1/…`); recursive `find "$root"/spec … "*/reports/…"` | Introduce `EVIDENCE_ROOT="$root/evidence"`; base-swap `spec/` → `evidence/` in every path incl. the 8 namespace dirs. `*/reports/…` sub-glob **preserved** (layout is 1:1). **Must return 119/119 in the same commit.** |
| `spec_lint.py` | 4-prefix folder resolution (`spec/`,`v1`,`v2`,`v3`); `check_anchors`/`check_trace` under `spec/`; `dead-link`+`orphan` walk `spec/`; `check_shipped_have_tests`; `status-drift`; **`feature-shipped-trace-drift`**; **`feature-shipped-changelog-missing`**; `KNOWN_FROZEN_DEAD_LINKS`, `CHANGELOG_ROLLUP_ALLOWLIST`, line-201/1018 report exceptions | **Re-found** (Phase 5b): feature-folder walk → `stories/`; anchor resolution → `evidence/`; the ADR-0082 **triad** re-expressed as **story-status (`sprint-status.yaml` + story frontmatter) ↔ `trace.toml` state ↔ CHANGELOG index**. Highest-risk software change; owns synthetic self-tests that must be rewritten to the new layout. |
| `spec_brief.py` | prefix resolution (`spec/`,`v1`,`v2`) | Repoint to `stories/` + `epics/`; or retire in favour of `bmad-sprint-status` (see §10). |
| `adr_registry_check.py` | path regex `spec/architecture/adr/*.md` ×3, README path | Repoint to `planning-artifacts/architecture/decisions/`. |
| `pre_stage_anchors.sh` | `SAMPLE_7D`/`SAMPLE_90D` = `spec/v1/operator-success-reports/reports/…` | Base-swap to `evidence/v1/operator-success-reports/reports/`. |
| `wave_b_emit.sh` | `REPORTS_DIR=…/spec/v5-…/reports` | Base-swap to `evidence/v5-…/reports`. |
| `capture_screenshot.sh` | `spec/v1/v0-paper-sma/reports/screenshots/` | Repoint to `evidence/v1/…/reports/screenshots/`. |
| `prune_backtest_duplicates.sh` | `anchors="$root/spec/anchors.toml"` | Move `anchors.toml` → `evidence/anchors.toml` (travels with the corpus); repoint. |
| `check_determinism_anchors.py` | `"spec/anchors.toml"` | Repoint to `evidence/anchors.toml`. |
| `precheck.sh` | `spec/$slug/tasks.md`, `spec/lumen-design-adoption/$slug/tasks.md` | Repoint to `stories/`; `tasks.md`→story `Tasks/Subtasks`. |
| `queue_staleness_check.py` | default `spec/backlog.md` | Repoint to `planning-artifacts/backlog.md`. |
| `operator_ledger_check.py` | `_DEVNOTE_RE = spec/dev-notes/…`, default ledger `spec/dev-notes/operator-side-pending-ledger.md` | Repoint regex + default to `docs/dev-notes/`. |
| `check_no_secrets_in_llm_artifacts.sh` | `--scan-spec` globs `spec/**.md`, `spec/**.toml` | Repoint globs to `_bmad-output/**` + `docs/**` + `evidence/**`. |

`anchors.toml` itself: its 9 `spec/` occurrences are **all comments** (provenance notes). Anchor **keys
are scenario names resolved by glob** — **zero functional edits**, comment-hygiene only. (The file does
`git mv` to `evidence/anchors.toml` so it travels with the corpus; that is a path move, not a key edit.)

### 5.2 Code — 19 load-bearing files (base-swap `spec/` → `evidence/` unless noted)

**Report walk / resolver (the corpus):**
- `crates/reports/src/parse.rs` — walks `spec/<feature>/reports/backtest-*.md` (the ≥9-anchor assertion). **Base-swap.**
- `crates/reports/tests/strategy_anchors_unchanged.rs` — `walk_collect` over `spec/**/reports/`; v5 dir refs. **Base-swap.**
- `crates/reports/tests/report_scenarios.rs` — publishes lock-copies to `spec/v1/operator-success-reports/reports/`; reads anchors. **Base-swap** (note: this walk feeds CI — see §5.3).
- `crates/ui/src/reports/loader.rs` — the report **picker** walks `spec/*/reports/`; test paths `spec/v1/v0-paper-sma/reports`. **Base-swap.**
- `crates/ui/src/lab/equity_loader.rs` — asserts `spec/` + `lab-runs/` are sibling roots; test `spec/v1/v1-cross-sectional-momentum/reports`. **Repoint `spec/`→`evidence/` in the sibling-root list.**

**Report emitters (CLI `default_value` / hardcoded output):**
- `crates/reports/src/bin/report.rs` — `default_value = "spec/v1/operator-success-reports/reports/report.md"`.
- `crates/agent/src/cron.rs` — `output_dir = "spec/v1/operator-success-reports/reports"` (default, operator-tunable).
- `crates/agent/src/kill_switch.rs` — `output_arg = "spec/v1/operator-success-reports/reports/incident-*.md"` (hardcoded).
- `crates/forecast/src/bin/{recalibrate_sigma_train,forecast_distribution,vol_verdict,regime_verdict,sharpe_comparison}.rs` — `default_value`/`PathBuf::from` report dirs.
- `crates/backtest/src/bin/{monte_carlo,param_robustness_sweep,run_yahoo_sma,threshold_sweep}.rs`, `crates/backtest/examples/passive_baseline_equity.rs`, `crates/backtest/tests/{determinism,multi_pair_determinism}.rs`, `crates/trader/src/bin/llm_verdict.rs` — grep-surfaced `spec/`-anchored output/default paths. **Base-swap.**
- `crates/ui/src/bin/viewer.rs` — sample report path `spec/v1/v05-composed-strategies/reports/backtest-*.md`.

**Runbook / knowledge paths (→ `docs/`, not `evidence/`):**
- `crates/ui/src/strings.rs` — `KILL_RUNBOOK_LINK_PATH = "spec/runbooks/kill-switch.md"` → `docs/runbooks/kill-switch.md`; `MODELS_EMPTY_STATE` doc ref.
- `crates/ui/src/baseline/loader.rs` + `crates/ui/tests/baseline_error_state.rs` — `spec/runbooks/artifacts/passive-baseline-2026-06-08` → `docs/runbooks/artifacts/…`.
- `crates/forecast/tests/patchtst_overlay_neutrality.rs` — reads a candidate list of `spec/v1/…/reports` dirs. **Base-swap.**

**Non-load-bearing:** ~470 `//!`/`///` rustdoc refs to `spec/<slug>/feature.md` etc. — documentation
links, no build/gate effect, **not** anchored (freely editable). Sweep them to the new story/PRD paths in
a **follow-up doc-hygiene pass** (out of the critical path; may run post-cutover).

### 5.3 CI interaction

`.github/workflows/ci.yml` contains **one** `spec/` reference — a comment. There is **no functional
`spec/` path in CI YAML**; CI touches `spec/` only *through* the scripts/tests it invokes
(`verify_anchors.sh`, `spec_lint.py`, `cargo test`). **Consequence:** the Phase 3 report move changes
what `crates/reports` tests (`report_scenarios.rs`, `strategy_anchors_unchanged.rs`) and the ui
`reports/loader.rs` tests **walk** — i.e. it changes CI *inputs*. CI is currently **red** (unrelated
run-2 shakeout, task open). The plan therefore:
- Does **not** assume green CI; each phase's gate recipe runs the affected gates **locally** before push.
- Flags Phase 3 as the one that alters CI inputs — validate `cargo test -p reports -p ui` locally against
  `evidence/` before the commit, and expect the existing shakeout reds to persist orthogonally.

---

## 6. The six agents → BMAD personas

| Our agent | BMAD persona / workflow | Fidelity | Where project knowledge lands |
|---|---|---|---|
| **analyst** | `bmad-agent-analyst` + `bmad-prd`, `bmad-market-research`, `bmad-domain-research` | **clean** | `_bmad/custom/bmad-agent-analyst.toml` (persistent_facts, principles, role) |
| **architect** | `bmad-agent-architect` + `bmad-architecture`, `bmad-create-architecture`, `bmad-create-epics-and-stories`, `bmad-check-implementation-readiness` | **clean** | `_bmad/custom/bmad-agent-architect.toml` |
| **developer** | `bmad-agent-dev` + `bmad-dev-story`, `bmad-create-story` | **clean** | `_bmad/custom/bmad-agent-dev.toml` |
| **ui-designer** | `bmad-agent-ux-designer` + `bmad-ux` | **clean** (BMAD ux is design-only; our ui-designer also writes Rust — fold that into dev-agent menu) | `_bmad/custom/bmad-agent-ux-designer.toml` |
| **tester** | **no persona agent** → `bmad-code-review` (fresh-context, ideally different LLM) + `bmad-qa-generate-e2e-tests`, driven by the **dev** persona, + our harness skills | **role-preserved-via-workflow** | `_bmad/custom/bmad-code-review.toml` (the test-report template contract + verify-before-route discipline) |
| **presenter** | `bmad-agent-pm` + `bmad-sprint-status` + `bmad-retrospective` (the sprint-review face) | **role-preserved** | `_bmad/custom/bmad-agent-pm.toml` |

**Auxiliary agents (not in the ratified "six", handled honestly):**
- **researcher** → `bmad-agent-analyst` + `bmad-{domain,market,technical}-research`. Largely **dormant**
  (the `research/` KB is complete, 900/900). Note-only.
- **spec-auditor** → **no BMAD equivalent.** Its job (orphan/stale/anchor-drift audit) *is* our machinery.
  **Keep** as a project-custom role that runs the re-founded `spec_lint` + `verify_anchors` and emits the
  weekly `docs/dev-notes/audit-*.md`. `bmad-sprint-status` covers *some* of the surface (risk callouts) but
  not anchor/trace drift.
- **ui-debugger** → **no clean equivalent.** Fold its iced render-verification ladder into
  `bmad-agent-dev`/`bmad-agent-ux-designer` via `persistent_facts` (`file:docs/dev-notes/iced-ui-render-verification.md`)
  and keep it as a project-custom specialization. **Keep.**

**Customization mechanism (official, per `bmad-customize`).** For each persona write a sparse
`_bmad/custom/bmad-agent-<role>.toml` that:
- appends `persistent_facts`: `file:{project-root}/_bmad-output/project-context.md` (non-negotiables),
  plus role-specific files (architect ← `file:…/planning-artifacts/architecture.md`; tester ←
  `file:…/bmad-code-review` test-report template; ui-* ← `file:docs/dev-notes/iced-ui-render-verification.md`);
- appends `principles` (e.g. architect: "day-1 baseline-divergence e2e test for every overlay";
  "Decimal money, never f64"; "boring production-proven crates");
- overrides `role`/`communication_style` from the `.claude/agents/*.md` prose;
- adds `[[agent.menu]]` items (matched by new `code`) that invoke **our harness skills** —
  `rust-build`, `rust-test`, `rust-validate`, `verify-anchors`, `backtest`, `cockpit-smoke` — so the
  personas can still drive the Rust gates.

After the `.toml` overrides exist and verify (Phase 5a), the `.claude/agents/*.md` files **retire**
(Phase 5c).

---

## 7. CLAUDE.md / AGENT.md cutover

Both are **rewritten around the BMAD cycle** — fresh-chat-per-workflow, `sprint-status.yaml`-driven,
story-based dev/review — while the **non-negotiables section is carried verbatim and stays in force**:

- FROZEN robustness gate; no ship on `REGRESSION` without human override.
- Byte-immutable anchored reports (ADR-0038 §D6); now under `evidence/**/reports/`.
- Day-1 baseline-equity-divergence e2e test for every overlay/sizing modifier.
- Render-PIXEL UI verification (the iced emulator harnesses).
- `Decimal` money + `Money<C>` newtype; never `f64`; exact-cent reconciliation.
- Determinism: `ChaCha20Rng::from_seed`; run-varying fields in YAML front-matter only.
- **do-not-build register** — the "check the register before proposing features" rule is **lifted into
  `project-context.md`** so every persona loads it on activation.
- ADR registry atomicity (write ADR + register in the same commit) — repointed to
  `planning-artifacts/architecture/decisions/`.

**File-precedence rewrite** (CLAUDE.md §"Where to start"): `product.md`→`PRD.md`;
`architecture.md`→`planning-artifacts/architecture.md`; `backlog.md`→`sprint-status.yaml` +
`planning-artifacts/backlog.md`; add `project-context.md` as the AI-rules entry point. `AGENT.md` §"The
six agents" + §"Canonical workflow" re-expressed as the BMAD persona cycle; §"Communication contract" +
the structured handoff envelope **kept** (BMAD has no equivalent hand-off schema).

---

## 8. Phasing — gates green at every commit

Each phase is one (or a few) commits; **content transform + machinery repoint travel together**. Gate
floor at every commit: `bash scripts/verify_anchors.sh` → **119/119** and `python3 scripts/spec_lint.py`
→ **PASS(0)**, plus `cargo build --workspace` where code moved. All work on `main`; orchestrator commits.

| Phase | Content | Machinery (same commit) | Gate recipe | Rollback |
|---|---|---|---|---|
| **0. Install** | Run installer; hand-tune `_bmad/*/config.yaml` (`project_name: trading`) | none (additive) | `verify_anchors`=119/119 (untouched); `spec_lint`=PASS; `cargo build` | `rm -rf _bmad _bmad-output docs .claude/skills/bmad-*` |
| **1. Planning docs** | Author `PRD.md` (from `product.md`), `architecture.md` + ADR annex mirror, `project-context.md`; run `bmad-document-project`→`docs/` | none — `spec/` still authoritative | gates unchanged (`spec/` intact) + read-review the generated docs | delete the new `_bmad-output`/`docs` files |
| **2. Epics + stories** | Generate 7 `epics/*.md`, ~120 stories (`done`), `sprint-status.yaml`; **copy** `trace.toml` to new path (not yet authoritative) | none | gates unchanged; `bmad-sprint-status` sanity-reads the yaml | delete generated stories/epics/yaml |
| **3. Move corpus + repoint anchors** ⚠ | `git mv spec/**/reports → evidence/**/reports`; `git mv spec/anchors.toml → evidence/anchors.toml` | `verify_anchors.sh` base-swap (+8 namespace dirs); `spec_lint` anchor-base; 19 code files; `pre_stage_anchors.sh`, `wave_b_emit.sh`, `capture_screenshot.sh`, `prune_backtest_duplicates.sh`, `check_determinism_anchors.py` | **`verify_anchors`=119/119** (the load-bearing proof); `cargo test -p reports -p ui -p forecast -p backtest`; `spec_lint`=PASS | single-commit `git revert` (mv back + revert edits) |
| **4. Move knowledge** | `git mv spec/{dev-notes,runbooks,design,ui-design-principles.md} → docs/…`; move do-not-build register + research-gap; **this plan file moves too** | `strings.rs` runbook path; `baseline/loader.rs` + test; `operator_ledger_check.py`; `check_no_secrets…sh` globs | `cargo test -p ui` (baseline/runbook tests); `spec_lint`=PASS; `operator_ledger_check.py`=PASS | `git revert` |
| **5a. Personas** | Author `_bmad/custom/bmad-agent-*.toml` + workflow overrides; fold `.claude/agents/*` prose | none (config only) | `resolve_customization.py --skill … --key agent` resolves each; smoke-invoke one persona | delete `_bmad/custom/*.toml` |
| **5b. Re-found lints + retire `spec/`** ⚠ | `git rm -r spec/` feature folders (content now in stories/epics/PRD/arch); make `trace.toml` authoritative at new path | **Re-found `spec_lint`**: triad = story-status ↔ trace ↔ CHANGELOG; walks `stories/`+`evidence/`; `spec_brief.py`, `precheck.sh`, `queue_staleness_check.py`, `adr_registry_check.py`; `spec_lint --self-test` rewritten | `spec_lint --self-test`=0; `spec_lint`=PASS; `verify_anchors`=119/119; `adr_registry_check.py --pre-commit`=0 | `git revert` (largest blast radius — do last, isolated) |
| **5c. Docs cutover** | Rewrite `CLAUDE.md`, `AGENT.md`, `README.md` around BMAD; `git rm -r .claude/agents` | doc-comment hygiene sweep (~470 refs, may defer) | `spec_lint`=PASS; `cargo build`; manual read-review | `git revert` |

**Watch recipe for the long gate (paste while Phase 3/5b run):**
```
watch -n 20 'bash scripts/verify_anchors.sh 2>&1 | tail -3; echo ---; python3 scripts/spec_lint.py 2>&1 | tail -2'
```

**Why this order is safe:** Phases 0–2 are purely additive (`spec/` stays authoritative, gates cannot
regress). Phase 3 is the only anchor-critical move and is a **layout-preserving base-swap** — the
`*/reports/…` glob and every namespace resolver are a pure string substitution, so 119/119 holds across
the commit. Phase 5b (lint re-founding + `spec/` deletion) is the highest-risk software change and runs
**last, isolated**, with its own self-test gate.

---

## 9. Risks

1. **`spec_lint` re-founding (Phase 5b) is real software** — the ADR-0082 triad + synthetic self-tests
   must be rewritten to the story/sprint-status layout. Mitigation: TDD the self-tests first against
   fixtures; isolate in its own commit; keep the old `spec/`-based lint runnable until the new one passes.
2. **Report move alters CI inputs (Phase 3)** on an already-red CI. Mitigation: validate affected
   `cargo test` targets locally against `evidence/` before push; treat pre-existing shakeout reds as
   orthogonal and documented.
3. **BMAD `uv run` dependency** — v6 skills increasingly shell out to `uv run …python`. Confirm `uv` is
   available in the agent environment or the workflows degrade to the documented manual-merge fallback
   (each SKILL.md ships one). Not a blocker; note for the operator.
4. **PRD/architecture reshape is lossy if rushed** — 57 KB product + 13 arch docs + 86 ADRs carry nuance.
   Mitigation: ADRs `git mv` verbatim (no reshape); PRD/architecture run through the BMAD **Update**-mode
   workflows with human review (Phase 1 gate is a read-review, not just a lint).
5. **Two write-paths during transition** — `spec-update` skill vs BMAD workflows. Mitigation: freeze
   `spec/` writes at Phase 3; route all new writes through BMAD thereafter.
6. **Anchored-body immutability during `git mv`** — a stray content edit during the move breaks 119/119.
   Mitigation: `git mv` only (never open the files); run `verify_anchors` immediately after the mv,
   before any script edit, to isolate move-vs-repoint failures.

---

## 10. Open operator decisions

1. **Evidence root location** — recommend top-level `evidence/` (sibling of `lab-runs/`, matches the
   ADR-0055 sibling-root model + minimal resolver churn). Alternative: `_bmad-output/evidence/` (keeps all
   migrated content under one tree). **Decision needed.**
2. **`trace.toml` fate** — recommend **keep** as a ported, machine-checked ledger (BMAD's FR-coverage-map
   is prose; the triad lint needs a real ledger). Alternative: derive it from story frontmatter and delete
   the file. **Decision needed.**
3. **Story granularity** — recommend **1:1 per feature, ~120 top-level + rollup sub-tasks, 7 epics**
   (§4.2). Alternative: thinner index. **Decision needed.**
4. **CHANGELOG role** — recommend **keep at root** as the human "done" index and the third triad leg.
   Alternative: let `sprint-status.yaml` be the sole done-index and drop CHANGELOG (loses the human
   narrative + a triad leg). **Decision needed.**
5. **`spec-brief` / `spec-update` skills** — `spec-brief` is superseded by `bmad-sprint-status` +
   story reads; `spec-update` by the BMAD write-path. Recommend: retire `spec-brief`; keep `spec-update`
   as a thin shim that writes to `_bmad-output/` until muscle-memory fades, then retire. **Decision needed.**
6. **Cutover cadence** — big-bang (all 7 phases in one sprint) vs staged over weeks with `spec/` and
   `_bmad-output/` coexisting. Recommend staged; note the two-write-path risk (§9.5). **Decision needed.**
7. **`uv` availability** for BMAD's Python steps (§9.3). **Confirm.**

---

## 11. What BMAD has NO equivalent for (ported, never dropped)

Full migration re-homes these; it does not delete a guarantee:

| Guarantee | BMAD equivalent | Disposition |
|---|---|---|
| Byte-SHA-256 **anchors** (119) + `verify_anchors.sh` | none | **Port** → `evidence/`, base-swap resolver |
| `trace.toml` requirement ledger (147) | prose FR-coverage-map only | **Port** → `planning-artifacts/trace.toml` |
| `spec_lint` triad (status ↔ trace ↔ CHANGELOG) + orphan/dead-link/status-drift | none | **Re-found** on story/sprint-status layout |
| **ADR registry** atomicity lint | none (BMAD has no ADR concept) | **Port** → `decisions/` annex + `adr_registry_check.py` repoint |
| `evidence/*/reports/` immutable corpus | none | **Port** (layout preserved) |
| spec-auditor drift audit | partial (`bmad-sprint-status` risk callouts) | **Keep** as project-custom role |
| ui-debugger render-pixel ladder | none | **Keep** folded into dev/ux personas |
| Structured hand-off envelope (AGENT.md) | none | **Keep** in rewritten AGENT.md |

---

## Appendix — provenance

- BMAD facts read from `…/scratchpad/bmad-probe/` (v6.10.0): `_bmad/bmm/config.yaml`,
  `_bmad/_config/manifest.yaml`, `.claude/skills/bmad-{customize,generate-project-context,document-project,
  sprint-planning,sprint-status,prd,create-epics-and-stories,create-story}/` (SKILL.md + templates +
  `customize.toml`).
- Toucher counts grep-grounded: `crates/` 488 total `spec/` refs, 31 load-bearing across 19 files;
  `scripts/` 13 files; `.github/` 1 comment; `spec/anchors.toml` 9 comment lines; `spec/trace.toml` 147
  `[[req]]`.
- Corpus: 155 `feature.md`, 130 `reports/` dirs, 119 anchors, 86 ADRs.
- Gate baseline at authoring: see the migration commit's `verify_anchors` (119/119) + `spec_lint`
  (PASS) run appended to the orchestrator's handoff.
