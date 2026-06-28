# CLEANUP-PLAN.md — Repository Audit & Cleanup Plan

> Read-only audit of `trading/` at HEAD `10d1709` (2026-06-11).
> PLAN ONLY — nothing has been deleted, moved, or modified. Every number below
> comes from commands actually run against the working tree / git index.

> **PHASE-3 RATIFICATIONS (operator, 2026-06-12):** **P3-2 = (a) KEEP the research-era Rust** (~30-35k LOC stays — it backs the 119 locked anchors; standing decision). **P3-4 = SKIP the history rewrite** (confirmed off the table; revisit trigger stays .git > 150 MB — currently 41 MB post-gc). P3-1 (retina screenshots) and P3-5 (vendor `.orig`) were NOT ratified — deferred, not rejected.
>
> **EXECUTION LOG (2026-06-11, operator-ratified "Phase 1 + 2"):**
> ✅ P1-1 `git gc` · ✅ P1-2 `cargo clean` + local debris · ✅ P1-3 tracked `.pyc` removed ·
> ✅ P1-4 `crates/models` stub removed (workspace check clean) · ✅ P1-5 `spec/_probe_lint_test/` removed ·
> ✅ P1-6 `SPEC_HYGIENE_PLAN.md` → dev-notes archive · ✅ P1-7 **47** un-anchored tester reports
> → `spec/archive/tester-reports-2026-05-to-06.tar.gz` (51 of the planned 98 were each their
> feature's ONLY test evidence — restoring them keeps the spec-lint `shipped-no-tests` contract
> intact; net new-finding count vs baseline = **0**) ·
> ✅ P2-1 108 presentation files → `presentations-2026-Q2.tar.gz` · ✅ P2-2 **49** dev-notes →
> `spec/dev-notes/archive/2026-Q2/` (13 load-bearing kept; `feature-triage` returned — cited by a
> restored report) · ✅ P2-3 backlog `## Recent` 1,405 lines → `archive/backlog-recent-2026-05.md`
> (de-linkified) · ✅ P2-4 45 design prototypes → `design-prototypes-2026-Q2.tar.gz`
> (`colors_and_type.css` kept — `theme.rs` provenance source).
> **Gates after:** anchors **119/119 PASS** · spec-lint **94 → 71** (zero NEW findings; 26
> pre-existing fixed incl. 2 trace rows) · `cargo check --workspace --all-targets` clean ·
> **zero** edits under `spec/*/reports/` except the 5 un-anchored screenshot READMEs + the
> 51 byte-identical restores. Phase 3 items remain operator decisions.
> Companion precedents already in-tree:
> [`spec/dev-notes/repo-cleanup-plan-2026-05-22.md`](spec/dev-notes/repo-cleanup-plan-2026-05-22.md)
> and [`spec/dev-notes/retired-surface-inventory-2026-05-22.md`](spec/dev-notes/retired-surface-inventory-2026-05-22.md).

---

## 1. Executive summary

| Metric | Value |
|---|---|
| Tracked files (`git ls-files`) | **1,879** |
| Working tree on disk | **20 GB** — of which `target/` (gitignored build cache) = **20 GB**; everything else ≈ 100 MB |
| `.git` | **56 MB** (objects 51.4 MB; packed only 28.65 MiB → ~22 MB loose; LFS 5 MB) |
| Rust | 652 tracked `.rs`, **222,454 raw lines** (150,231 code per `loc`) |
| Markdown | **783 tracked `.md`, ≈ 289,000 lines** (spec/ alone: 283,413) |
| Binary assets tracked | PNG 29.5 MB, safetensors 5 MB (LFS), logs/txt/csv/json ~1 MB, snapshots 552 KB |

**Top 5 size/LOC contributors**

1. `target/` — 20 GB local build cache (not in git).
2. `crates/ui` — 67.5k raw Rust lines (30 % of all Rust).
3. Feature-folder markdown (`spec/<slug>/feature.md|tasks.md|…`) — 225 files / 158,516 lines (55 % of all markdown).
4. Tracked PNGs — 29.5 MB (15.3 MB load-bearing visual baselines + 14.3 MB spec screenshots, of which 5 retina screenshots in `spec/v1/chart-canvas-overhaul/` = 11.6 MB).
5. `crates/backtest` — 37.5k raw Rust lines (incl. the 4,382-line `param_robustness_sweep.rs` research binary).

**The 5 biggest cleanup levers**

| # | Lever | Est. savings | Class |
|---|---|---|---|
| 1 | `cargo clean` (or prune old `target/` profiles) | **~20 GB disk** (local only; cost = one full rebuild) | SAFE-NOW |
| 2 | Archive un-anchored historical markdown: 98 tester reports (24,011 lines), 77 presentations (23,602 lines), ~35 stale dev-notes (~15k lines), backlog "Recent" cohorts (~3.5k lines) | **~210 files / ~66k md lines (-23 % of markdown), ~2 MB** | SAFE-NOW (with spec-lint pass) |
| 3 | `git gc --aggressive` + drop 11.6 MB chart-canvas diagnostic screenshots | **.git 56→~34 MB now; tree -12 MB**; full history rewrite could reach ~20 MB but is DESTRUCTIVE | gc SAFE-NOW / screenshots & rewrite NEEDS-OPERATOR-DECISION |
| 4 | Archive concluded research-era Rust (montecarlo, carry/funding/basis/MN-spread, robustness sweeps, retired forecasters) | **~30–35k LOC (~15 % of Rust)** — but it underpins 119 locked anchors | NEEDS-OPERATOR-DECISION |
| 5 | Root + micro debris: unused `models` stub crate, `spec/_probe_lint_test/`, tracked `.pyc`, stale `SPEC_HYGIENE_PLAN.md`, local `test_field_method2/` + `librust_out.rlib` | small (~1 MB, ~500 lines) but kills recurring audit noise | SAFE-NOW |

**Answers to the operator's five questions** are embedded per section: §2 (Q4 languages), §3 (Q2 Rust), §4 (Q3 markdown), §5+§6 (Q5 size), and the KEEP/REMOVE verdicts throughout (Q1).

---

## 2. Language inventory (Q4 — "do we need all these languages?")

`loc --exclude target` over the working tree:

| Language | Files | Code lines | Where it lives | Verdict |
|---|---|---|---|---|
| Markdown | 749 | 242,257 | `spec/` (97 %), `.claude/`, root | KEEP core / ARCHIVE long tail — see §4 |
| Rust | 647 | 150,231 | `crates/*` (98.5 %), `vendor/` (1.5 %) | KEEP — see §3 |
| TOML | 58 | 5,591 | Cargo manifests, `spec/trace.toml` (328 KB / 97 req-rows), `spec/anchors.toml` (119 anchors), config | KEEP — gate- and build-bearing |
| Python | 7 | 2,383 | `scripts/` only: `hash_report.py`, `check_determinism_anchors.py`, `spec_lint.py`, `spec_brief.py`, `adr_registry_check.py`, `operator_ledger_check.py`, `queue_staleness_check.py` | **KEEP — load-bearing.** These ARE the anchor + spec-lint gates. A Rust rewrite is possible (REPLACE option, §6 P3-6) but buys no size: 2.4k lines of Python would become ≥ similar Rust + build time |
| JSX | 17 | 1,400 | `spec/design/` (Lumen design-system prototypes) | ARCHIVE-CANDIDATE — static design references, never built/run by any pipeline |
| CSS | 3 | 1,135 | `spec/design/`, `crates/audit` fixture | KEEP audit fixture; design CSS goes with JSX |
| Shell | 17 | 854 | `scripts/` — `verify_anchors.sh`, cockpit-smoke, orchestrator probes | KEEP — gate-bearing (`verify_anchors.sh` re-verified today: **119/119 PASS**) |
| HTML | 22 | 823 | `crates/audit` test fixtures (12), `spec/design/`, `visual-fail-html-reporter` sample | KEEP fixtures; design HTML archivable |
| SQL | 15 | 195 | `crates/audit` (SQLite schema/migrations) | KEEP — audit ledger is product code |
| Plain text / JSON / CSV / logs | ~60 | ~700 | mostly `spec/*/reports/` evidence artifacts | ARCHIVE with their reports (§4) |
| Swift | 1 | 21 | `scripts/orch_cursor_move.swift` (macOS screenshot orchestration) | KEEP — 21 lines, used by capture tooling |

Tracked-files-only view (`git ls-files` by extension): 783 md, 652 rs, 138 snap, 70 png, 59 toml, 27 log, 23 txt, 22 html, 17 sh, 17 jsx, 15 sql, 10 csv, 7 py, 7 json, 4 svg, 3 safetensors, 3 gz, 1 each parquet / db / rgba / pyc / orig.

**Bottom line:** no language is freeloading. Python+shell = the regression gate; SQL/HTML = audit fixtures; Swift = 21 lines. The only removable "language mass" is the JSX/HTML/CSS design prototypes (~3.4k lines) and that is an archive question, not a tooling question. `target/` (20 GB) is local-only and never pollutes git.

---

## 3. Rust breakdown (Q2 — "are 150k+ lines all necessary?")

Raw line counts (`wc -l`, includes comments/blank; `loc` code-only total is 150,231):

| Crate | src | tests | benches | Total | Note |
|---|---:|---:|---:|---:|---|
| ui | 45,262 | 21,529 | 670 | **67,461** | Cockpit — product-active (Live view parked in TODO.md) |
| backtest | 22,083 | 8,666 | 0 | **30,749** | incl. `bin/` 6,727 (research binaries) + `scenarios/` 5,626 |
| forecast | 16,103 | 4,593 | 0 | **20,696** | TCN/PatchTST/GARCH — retired-but-anchored research |
| strategy | 12,623 | 2,374 | 551 | 15,548 | prod strategies + research overlays |
| data | 11,432 | 1,718 | 74 | 13,224 | fetchers incl. funding/basis spikes |
| audit | 6,590 | 7,143 | 55 | 13,788 | trade ledger — product core |
| agent | 7,220 | 4,431 | 245 | 11,896 | runtime — product core |
| llm | 7,769 | 3,304 | 0 | 11,073 | LLM forecaster (shipped-partial) |
| reports | 5,144 | 6,176 | 0 | 11,320 | report rendering — anchor-gated |
| trader | 4,778 | 4,084 | 0 | 8,862 | product core |
| core | 3,946 | 346 | 0 | 4,292 | product core |
| reflection | 2,141 | 1,590 | 156 | 3,887 | shipped |
| features | 2,328 | 0 | 0 | 2,328 | shipped |
| cost | 1,045 | 136 | 0 | 1,181 | 7 consumers |
| exec | 672 | 105 | 168 | 945 | 2 consumers |
| replay-cache | 682 | 0 | 0 | 682 | used by forecast |
| risk | 612 | 0 | 0 | 612 | 2 consumers |
| **models** | **5** | 0 | 0 | **5** | **v0 stub, ZERO consumers** → REMOVE |
| **Sum crates** | 150,435 | 66,195 | 1,919 | **218,549** | |
| vendor/iced_tiny_skia | | | | **3,366** (14 files, 176 KB) | KEEP-LOCKED |
| root `src/` | | | | 0 | does not exist (CLAUDE.md repo map is stale here) |

### (a) Vendored fork — KEEP-LOCKED

`vendor/iced_tiny_skia/`: 3,366 lines / 176 KB. Operator-locked 2026-05-20, carries the upstream canvas-clip fix (`76b32d4906`). **Cannot be deleted while iced 0.14 is in use.** Retirement path: a future iced upgrade that includes the clip fix, with the mandatory `Transformation::scale(...) * group.transformation()` ordering audit per `spec/v1/chart-fixture-line-clipping/feature.md`. One nit inside it: `vendor/iced_tiny_skia/Cargo.toml.orig` is patch debris, not upstream source — 1-file removal, but vendor/* changes are contractually out of scope, so NEEDS-OPERATOR-DECISION (trivial).

### (b) Research-era code — the program is CONCLUDED, the code is the evidence

The active-vs-passive program was closed 2026-06-08: backlog `## Active` carries the terminal verdict — *"PROGRAM CONCLUDED — ACTIVE-EDGE SEARCH CLOSED, SHIP PASSIVE"* — and "ship passive" is explicitly defined as *promotion of already-built+anchored code*, not a build. Measured research surface (grep-identified, raw lines):

| Cluster | LOC | Files |
|---|---:|---|
| Robustness sweep harness | 4,382 | `crates/backtest/src/bin/param_robustness_sweep.rs` |
| Monte-Carlo bootstrap | 4,234 | `scenarios/montecarlo.rs` 1,095 · `bin/monte_carlo.rs` 960 · `data/src/synth/bootstrap.rs` 1,314 · `tests/montecarlo_e2e.rs` 865 |
| Cross-sectional momentum/MR | 3,802 | `strategy/src/cross_sectional/*` 3,483 · `features/src/cross_sectional.rs` 319 |
| Pairs / MN-spread | 3,000 | `strategy/src/pairs/*` 1,656 · `scenarios/pairs.rs` 428 · `tests/mn_spread_divergence_e2e.rs` 604 · misc |
| Carry / funding / basis | 4,425 | `backtest/src/{basis,funding}_data.rs` 1,462 · carry+basis e2e tests 1,510 · `data/src/bin/fetch_binance_funding.rs` 795 · `data/examples/basis_diag.rs` 595 · misc |
| Misc (benches, diag, lab markers) | ~1,500 | |
| **Subtotal research-strategy** | **~21,400** | |
| Retired forecaster surfaces (v25 TCN/PatchTST, v3 vol/GARCH/XGBoost — largely `crates/forecast` + strategy overlays) | **13,889** | per `retired-surface-inventory-2026-05-22.md` (some overlap with rows above) |

**Combined: ~30–35k LOC ≈ 15 % of all Rust.** Verdict: **NEEDS-OPERATOR-DECISION**, never SAFE-NOW, because:

- The retired-surface inventory's own conclusion: *"even retired code is on the regression gate… none of the 13,889 LoC inventoried is dead code in the silent-rot sense."* These paths produce the bodies behind a large share of the **119 locked anchors** (verified 119/119 PASS today).
- Deleting them = trading **reproducibility of ratified research conclusions** for ~15 % LOC. The anchors would have to be migrated to a "frozen evidence" namespace or retired via an ADR-0038-style re-baseline — an architect+operator protocol, not a cleanup commit.
- Honest options: **(i) keep as-is** (costs nothing at runtime; compile time only), **(ii) feature-gate research binaries/scenarios out of default builds** (keeps anchors re-runnable, cuts default build), **(iii) archive-and-delete with anchor re-baseline ADR** (max savings, loses one-command reproducibility — recoverable from git history only).

### (c) Dead/unused-code pass (time-boxed)

- `cargo check --workspace` → **0 warnings** (clippy `-D warnings` discipline holds; nothing surfaces for free).
- `#[allow(dead_code)]`: **71 occurrences** (file spread: ui 14, backtest 5, strategy 4, reports 4, forecast 4, data 2, …) — each one is a suppressed lead worth an audit pass.
- `crates/models`: 5-line v0 stub, **no consumer anywhere** (`grep` across all Cargo.tomls) — REMOVE (workspace-member edit + folder).
- All other small crates (risk, exec, cost, replay-cache) have real consumers — KEEP.
- **Proper follow-up tooling (not installed today): `cargo machete` (unused deps, fast) and `cargo +nightly udeps` (precise).** Recommend installing cargo-machete first; one run will settle the Cargo.toml-dependency question this pass could not cheaply answer.

---

## 4. Markdown inventory (Q3 — 783 files, "we don't need them all")

783 tracked `.md` ≈ 289k lines. By area:

| Group | Files | Lines | Verdict | Rationale |
|---|---:|---:|---|---|
| Root: README, CLAUDE, AGENT, TODO | 4 | 1,353 | **KEEP** | Operating contract + active TODO |
| Root: `SPEC_HYGIENE_PLAN.md` | 1 | 401 | **ARCHIVE** | 2026-05-13 proposal, since implemented (spec-lint/spec-brief/trace.toml/architecture-split all exist); its own frontmatter says it belongs in `spec/dev-notes/` |
| spec core depth-1 (`backlog.md` 5,114 · `product.md` 1,011 · `ui-design-principles.md` 767 · `bug-log.md` 196 · `architecture.md` 178) | 5 | 7,266 | **KEEP + PRUNE backlog** | `backlog.md` is the 2nd-largest md file in the repo; `## Recent (shipped)` cohorts (≈ lines 3,311–4,716) can move to `spec/archive/` |
| `spec/architecture/` + ADRs | 66 | 17,048 | **KEEP** | Live system design + decision record; gate scripts reference it |
| Feature folders — `feature.md`/`tasks.md`/decomp etc. (~102 folders) | 225 | **158,516** | **KEEP shipped/active; ARCHIVE-CANDIDATE the 9 deprecated/retired folder docs** | This is 55 % of all markdown. Heaviest: `lumen-design-adoption` 19,446 lines/26 files, `v2-llm-strategy` 6,420, `v3-llm-forecaster` 4,869. Statuses: 69 shipped, 5 deprecated, 4 retired, 5 tester-done, 5 dev-done, 4 presenter-done, rest singles. Deprecated/retired set: `_probe_lint_test`, `cockpit-chart-cache`, `v25-dl-forecast-overlay`, `v25b-transformer-overlay`, `v26-forecast-bakeoff`, `carry-strategy`, `v3-volatility-forecaster{,-rebaseline}`, `v3-xgboost-cheap-classifier`. CAUTION: retired research folders also host anchored reports — archive the *narrative* files only, never `reports/` |
| `spec/*/reports/` | 290 | 40,448 | **SPLIT** | **162 files match a locked anchor scenario → KEEP-LOCKED (byte-immutable, ADR-0038 §D6).** The ~98 `test-*.md` tester reports (24,011 lines) are *never* anchor-resolved (`verify_anchors.sh` only resolves `backtest-*`/`success-*`/`robustness*`/`<scenario>-<stamp>` names) → ARCHIVE per existing precedent (`spec/archive/pre-lumen-tester-reports-2026-04-to-05-03.tar.gz`). Plus ~30 misc evaluation/diag files case-by-case. The 4 `v5-latency-slippage-sim-v0.X.0-*` migration folders are hardcoded in `verify_anchors.sh` → KEEP-LOCKED |
| `spec/*/presentations/` | 77 | 23,602 | **ARCHIVE** | Sprint-review decks of long-shipped features; operator already approved them; zero gate references. Keep only decks of not-yet-approved work (none currently pending) |
| `spec/dev-notes/` | 70 | 29,649 | **ARCHIVE ~half (~35 files / ~15k lines)** | 12 weekly `audit-*.md` (each superseded by the next), 6-file `bug-64-*` investigation chain (bug fixed), superseded scoping/fork notes. Precedent + destination already exist: `spec/dev-notes/archive/2026-Q2/`. KEEP load-bearing ones: `feature-state-table-2026-05-22.md`, `v3-vol-overlay-noop-discovery` (cited by CLAUDE.md non-negotiable), terminal-verdict-adjacent notes |
| `spec/design/` | 5 | 863 (+41 jsx/html/css files) | KEEP md / ARCHIVE prototypes | Lumen tokens are referenced; static JSX mockups are not |
| `spec/runbooks/` | 6 | 1,232 | **KEEP** | incl. `passive-baseline.md` — THE ship artifact of the concluded program |
| `spec/archive/` | 2 | 1,077 | KEEP | It is the archive |
| `.claude/` (7 agents + 14 skills + helpers) | 25 | 3,051 | **KEEP** | Operational config, actively invoked |

**Net markdown answer:** ~430–450 files are load-bearing (gates, contracts, anchored evidence, active specs). **~330 files (~66k lines) are archive/delete candidates** — tester reports, presentations, stale dev-notes, backlog cohorts, deprecated-folder narratives. Git history preserves all of it; the in-tree `tar.gz` archive pattern additionally keeps them greppable offline.

---

## 5. Other assets

| Asset | Size | Verdict |
|---|---|---|
| `crates/ui/tests/visual-baselines/**` PNGs (56 of 70 PNGs) | 15.3 MB | **KEEP — load-bearing.** They gate rendering (the Live-view saga precedent: verify UI at the render layer). Marked `binary` in `.gitattributes` |
| `spec/v1/chart-canvas-overhaul/reports/screenshots/` | **11.6 MB in 5 retina PNGs** (2.2–2.4 MB each, 3360×1890) + ~2.7 MB more | ARCHIVE/OPTIMIZE — diagnostic/presentation artifacts. **No anchored backtest report references any screenshot** (grep-verified); only an un-anchored `test-*.md` links them. Options: move into a `tar.gz` in `spec/archive/`, or downscale/oxipng in place (lossless ~30–60 %) |
| `spec/design/project/` PNGs | ~1 MB | goes with design archive |
| `crates/forecast/checkpoints/anchors/*.safetensors` | 5 MB (3 files, **LFS-tracked**) | **KEEP** — TCN/PatchTST anchor checkpoints; anchored reports re-run against them |
| 138 `.snap` files | 552 KB | KEEP — test goldens |
| 27 `.log` + 23 `.txt` + 10 `.csv` + json.gz | ~1 MB | ARCHIVE with their parent reports (cockpit-smoke logs etc.) |
| `crates/llm/fixtures/replay-v1.db`, yahoo sample parquet, `lumen-mark-64x64.rgba` | <1 MB | KEEP — test fixtures |
| `scripts/__pycache__/spec_lint.cpython-310.pyc` | 8 KB | **REMOVE from index** — tracked bytecode; `.gitignore` already covers `__pycache__/` but the file predates the rule |
| Data dirs | `data/` = 29 MB on disk, **only 5 `REVISION.toml` pins tracked** (verified `git ls-files data/`) — parquets correctly gitignored per ADR-0040 | KEEP policy as-is; local parquets are re-fetchable from the pins |
| Local debris (untracked, already gitignored) | `test_field_method2/` 452 KB, `librust_out.rlib` 8 KB | local `rm` — zero git impact |

**Largest git blobs** (`git rev-list --objects --all | git cat-file --batch-check | sort by size`): the top 6 are the chart-canvas screenshots (2.46, 2.41, 2.34, 2.26, 2.24, 0.99 MB); then **8 historical revisions of `charts_screen_dark_operator.png` (~6.3 MB history-only)** and 4 of `live__recent_activity_with_chevron__operator.png` (~3 MB) — visual baselines churn on every re-baseline and old versions stay in history forever.

**History-only opportunity:** pack is 28.65 MiB; loose objects ~22 MB (reflog/recent work) — `git gc` reclaims those non-destructively (56 → ~34 MB). Purging superseded baseline/screenshot revisions from history via `git-filter-repo` could shrink the pack toward ~15–20 MB, **but rewriting history is DESTRUCTIVE (breaks clones/reflogs) and is an operator-only decision** — at 56 MB total it is probably not worth it yet; revisit if baseline churn continues.

---

## 6. THE PLAN

Gates that must pass after **every** phase:
`cargo test --workspace` (known pre-existing reds, unrelated: `ui::lab_run_engine::h3` + 2 backtest montecarlo carry/funding tests) · `scripts/verify_anchors.sh` → **119/119 PASS** · `scripts/spec_lint.py` (no new dead links) · UI visual-baseline tests · `cockpit-smoke` after any UI-adjacent change.

### Phase 1 — SAFE-NOW (one session, zero gate risk)

| # | Action | Scope | Savings | Risk |
|---|---|---|---|---|
| P1-1 | OPTIMIZE | `git gc` (no history rewrite) | .git 56 → ~34 MB | None — content-preserving |
| P1-2 | REMOVE (local, untracked) | `target/` via `cargo clean` — or at minimum stale profiles; `test_field_method2/`; `librust_out.rlib`; `scripts/__pycache__/` dir | **~20 GB disk** | None to git; cost = full rebuild (~tens of minutes) |
| P1-3 | REMOVE (tracked) | `git rm --cached scripts/__pycache__/spec_lint.cpython-310.pyc` | hygiene | None — regenerated on every run |
| P1-4 | REMOVE | `crates/models/` + its `Cargo.toml` workspace-member line | 5 LOC, 1 crate slot | None — zero consumers (grep-verified); gate: `cargo test --workspace` |
| P1-5 | REMOVE | `spec/_probe_lint_test/` (2 stub files, May-13 lint probe) | kills a recurring spec-audit punch-list item | None — referenced only BY audit notes as a finding; gate: `spec_lint.py` |
| P1-6 | ARCHIVE | `SPEC_HYGIENE_PLAN.md` → `spec/dev-notes/archive/2026-Q2/` (via `spec-update` skill) | 401 lines off root | None — implemented long ago |
| P1-7 | ARCHIVE | The ~98 un-anchored `spec/*/reports/test-*.md` tester reports + tracked cockpit-smoke `.log`/`.txt` evidence → one `spec/archive/tester-reports-2026-05-06.tar.gz` (follow the existing pre-lumen precedent exactly) | ~100 files / 24,011 lines / ~1.5 MB | Low — **never anchor-resolved** (verified against `verify_anchors.sh` resolution rules); MUST exclude anything matching an anchored scenario name; run `verify_anchors.sh` + `spec_lint.py` after |

### Phase 2 — Low-risk archival sweep (needs only spec-lint diligence)

| # | Action | Scope | Savings | Risk |
|---|---|---|---|---|
| P2-1 | ARCHIVE | All 77 `spec/*/presentations/` decks (+ small `artifacts/`, 760 KB) of shipped features → `spec/archive/presentations-2026-Q2.tar.gz` | 77 files / 23,602 lines | Low — post-approval artifacts; fix inbound links flagged by spec-lint |
| P2-2 | ARCHIVE | ~35 stale dev-notes: 11 of 12 `audit-*.md` (keep newest), the 6 `bug-64-*` chain, superseded scoping/fork notes → `spec/dev-notes/archive/2026-Q2/` | ~15k lines | Low — keep `feature-state-table`, noop-discovery (CLAUDE.md-referenced), terminal-verdict notes |
| P2-3 | OPTIMIZE | Prune `spec/backlog.md` `## Recent (shipped)` cohorts into `spec/archive/backlog-recent-2026-05.md` | ~3.5k lines (5,114 → ~1.5k) | Low — pure move; spec-lint guards links |
| P2-4 | ARCHIVE | `spec/design/` JSX/HTML/CSS prototypes (41 files, ~3.4k lines) | ~3.4k lines | Low — static mockups; Lumen tokens (md) stay |

### Phase 3 — NEEDS-OPERATOR-DECISION (each item is a named trade-off)

| # | Action | Scope | Savings | The decision |
|---|---|---|---|---|
| P3-1 | ARCHIVE/REMOVE | `spec/v1/chart-canvas-overhaul/reports/screenshots/` 5 retina PNGs (11.6 MB) + remaining ~2.7 MB | **-12–14 MB tree** (history keeps them) | Visual evidence of a shipped UI feature, linked from an un-anchored tester report. Tar-archive vs lossless-optimize vs keep |
| P3-2 | ARCHIVE (code) | Research-era Rust: robustness sweep bin (4,382), montecarlo cluster (4,234), cross-sectional (3,802), pairs/MN-spread (3,000), carry/funding/basis (4,425) + retired forecaster surfaces (13,889, overlapping) | **~30–35k LOC (~15 % of Rust)** | **Size vs reproducibility of 119 anchored, operator-ratified research conclusions.** Options: (a) keep as-is — recommended default; (b) feature-gate out of default build — middle path, keeps anchors re-runnable; (c) delete + anchor re-baseline ADR — max savings, history-only reproducibility. Never SAFE-NOW per CLAUDE.md |
| P3-3 | ARCHIVE | Narrative md of the 9 deprecated/retired feature folders (NOT their `reports/`) | ~10–15k lines | Retired ≠ worthless: they document why lines of research closed. Cheap to keep; archive only if §4 sweep feels insufficient |
| P3-4 | OPTIMIZE (DESTRUCTIVE) | `git-filter-repo` purge of superseded visual-baseline + screenshot blob revisions | .git → ~15–20 MB | History rewrite breaks all clones; at 56 MB, likely **not worth it yet** — size the trigger at .git > 150 MB |
| P3-5 | REMOVE | `vendor/iced_tiny_skia/Cargo.toml.orig` (patch debris) | 1 file | vendor/* is operator-locked territory; trivial but needs the nod. The fork itself: **KEEP-LOCKED** (3,366 LOC) until an iced upgrade passes the clip-fix audit |
| P3-6 | REPLACE (option, not recommended now) | Rewrite `scripts/*.py` gates in Rust | -2.4k Python, +≥2.5k Rust | Honest sizing: no net size win, real regression risk on the anchor gate. Only worth it if Python becomes an env burden |
| P3-7 | OPTIMIZE | Audit the 71 `#[allow(dead_code)]` sites + run `cargo machete` / `cargo udeps` after installing them | unknown (est. 1–3k LOC + a few deps) | Follow-up tooling session; today's pass found 0 compiler warnings |

### Explicit KEEP-LOCKED register (do not touch, ever, without protocol)

- `vendor/iced_tiny_skia/` — operator-locked fork (clip fix).
- The **162 anchor-matching report files** under `spec/*/reports/` + `spec/anchors.toml` + `spec/trace.toml` — byte-immutable gate inputs (ADR-0038 §D6).
- `spec/v5-latency-slippage-sim-v0.{2,3,4,5}.0-*/` — directories hardcoded in `verify_anchors.sh` resolution.
- `scripts/verify_anchors.sh`, `scripts/hash_report.py`, `scripts/check_determinism_anchors.py`, `scripts/spec_lint.py` — the gates themselves.
- `crates/ui/tests/visual-baselines/**`, `*.snap`, LFS safetensors, test fixtures (`replay-v1.db`, yahoo sample parquet).
- `data/*/REVISION.toml` pins (incl. the stray local modification on `data/yahoo/REVISION.toml` — left untouched by this audit).

### Expected end-state if Phase 1+2 land and operator ratifies P3-1

- Tracked files: 1,879 → **~1,550**
- Markdown: 783 files / 289k lines → **~450 files / ~220k lines**
- Working tree (excl. target): ~100 MB → **~85 MB**; with `cargo clean`: 20 GB → < 1 GB
- `.git`: 56 MB → **~34 MB** (gc only)
- Rust: unchanged until the P3-2 operator decision (then optionally −15 %)
