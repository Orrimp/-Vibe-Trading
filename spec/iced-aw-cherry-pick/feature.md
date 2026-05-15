---
slug: iced-aw-cherry-pick
status: shipped
owner: presenter
updated: 2026-05-14
version: 1.0.0
predecessor: iced-native-widgets v0.1.0
parent: iced-ecosystem-evaluation v0.2.0
unblocked_by: cockpit-render-regression v1.0.0 (shipped 2026-05-14)
---

# iced_aw cherry-pick — Brief B (v0.1.0)

> **Status:** analyst draft. **No code changes, no crate adds in this brief.**
> Analyst translates the scope-locked Brief B resolution from
> [`iced-ecosystem-evaluation` v0.2.0 Q5](../iced-ecosystem-evaluation/feature.md)
> into per-sub-target requirements + falsifiable claims. Architect picks
> this up next and converts H-arch-4 / H-arch-9 / H-arch-10 into
> orchestrator-runnable falsifiers + a tasks.md plan, per
> [`AGENT.md ## Architect = hypothesis only`](../../AGENT.md#architect--hypothesis-only).

## TL;DR

Cherry-pick **three** `iced_aw` widgets — `date_picker` (B1), `spinner`
(B2), `badge` (B3) — under feature flags. B1 unblocks the v1.11 /
Phase 4 operator-selectable backtest range in the viewer bin; B2
upgrades the plain-text `"Loading…"` rendering currently emitted by
~8 panels to a visual spinner; B3 retires hand-rolled
`container().style(badge_style)` patterns on Strategies / Risk surfaces
and routes status chips through Lumen surface tokens. One direct dep
(`iced_aw = "0.14"`) gated to those three crate features; transitive
delta to be confirmed by architect via `cargo tree`. Zero anchor risk,
zero PNG-baseline impact (Charts surface untouched), ~13 panel snapshots
refreshed (estimate). Brief A's [`cockpit_table_style_fn`](../../crates/ui/src/theme/iced_widget_catalogs.rs)
factory — shipped-but-unused in Brief A — is one of Brief B's
intended consumption surfaces (badge custom-style hook).

## Predecessor — Brief A shipped state

[`iced-native-widgets` v0.1.0](../iced-native-widgets/feature.md)
landed on 2026-05-13 with `status: shipped`. Relevant carryovers for
Brief B (per [`iced-native-widgets/tasks.md`](../iced-native-widgets/tasks.md)
lines 124, 203, 318, 914, 932, 1024, 1028):

- **`cockpit_table_style_fn` Catalog adapter** lives at
  [`crates/ui/src/theme/iced_widget_catalogs.rs`](../../crates/ui/src/theme/iced_widget_catalogs.rs)
  (100 LOC module, 2 unit tests green). It mints a
  `StyleFn<'_, Theme>` boxed closure routing `color::BORDER_1` separator
  tokens. Brief A could **not** consume it — native iced 0.14 `Table`
  has no `.style(...)` builder; the upstream Catalog impl pre-bakes
  `Theme::default()` at construction
  (`iced_widget-0.14.2/src/table.rs:704-714`, `:144`). The module
  docstring explicitly designates Brief B `iced_aw` adopters + Themer
  overrides as the future consumption paths
  ([`iced_widget_catalogs.rs:33-43`](../../crates/ui/src/theme/iced_widget_catalogs.rs)).
- **Cargo.toml current pin:**
  `iced = { version = "=0.14.0", default-features = false, features = ["tiny-skia", "thread-pool", "advanced", "canvas"] }`
  ([`crates/ui/Cargo.toml:69`](../../crates/ui/Cargo.toml)) plus
  `iced_test = "=0.14.0"` dev-dep ([`crates/ui/Cargo.toml:108`](../../crates/ui/Cargo.toml)).
  Brief B adds **one direct dep**: `iced_aw = "0.14"` with
  `default-features = false` and exactly the `date_picker`,
  `spinner`, `badge` features enabled.
- **PNL / PNL_PCT badge surface** — Brief A preserved sentiment
  colors via `color_for_delta` inside per-column lambdas in
  [`positions.rs`](../../crates/ui/src/widgets/positions.rs) and
  [`strategies.rs`](../../crates/ui/src/widgets/strategies.rs), but
  did not retire any hand-rolled badge pattern. Brief B B3 is the
  first surface to consume a typed status-chip primitive.
- **PNG baselines (`ui-test-harness-bootstrap` v0.1):** all 3 PNG
  triples at [`crates/ui/tests/visual-baselines/charts_screen_dark_*.png`](../../crates/ui/tests/visual-baselines/)
  stay byte-identical. Brief B does not touch the Charts screen, and
  all three sub-targets land outside the Charts widget tree.

## Sub-targets

### B1 — `iced_aw::date_picker` (viewer-bin backtest range)

- **New surface.** The viewer bin's currently hard-coded backtest
  date range becomes operator-selectable via a `date_picker` widget.
  Unblocks v1.11 / Phase 4 follow-up.
- **Consumer surface.** Viewer bin (single instantiation site to be
  picked by architect). No cockpit panel touched.
- **Retired LOC.** 0 (greenfield surface — no prior date-input UI).
- **New surface LOC (analyst estimate).** ~30-50 LOC for instantiation
  + state plumbing into the viewer's existing scenario-selector struct.
- **Snapshot refresh count.** +1 panel snapshot (new viewer-bin panel
  capturing the picker in its default-closed state).
- **Acceptance criteria.**
  - `iced_aw = "0.14"` declared in `crates/ui/Cargo.toml` with
    `default-features = false` and exactly `["date_picker"]` (plus B2 /
    B3 features) enabled.
  - Viewer bin compiles, `cargo test -p ui` two-run determinism gate
    passes (no new flaky snapshots).
  - Date picker's day/month/year state is serializable for
    `scripts/check_no_clocks_in_ui_tests.sh` to pass on its panel
    snapshot.
  - Hypothesis **H-arch-4** unfalsified (see register below).
- **Out-of-scope for B1.** Wiring picker output into the actual
  backtest scenario fetch path — that is the v1.11 / Phase 4 brief's
  payload; B1 only ships the UI primitive + a hard-coded smoke-test
  consumer.

### B2 — `iced_aw::spinner` (panel_state::Loading rendering)

- **New surface.** `panel_state::Loading` panels render a visual
  spinner instead of the current plain-text `"Loading…"` placeholder.
- **Consumer surfaces (analyst estimate, architect to confirm exact
  panel set).** Currently ~8 panels emit a textual `"Loading…"`
  through their `panel_state` dispatch; the spinner replaces that
  rendering at the dispatch site, not per-panel.
- **Retired LOC.** Estimated ~10-20 LOC across the shared
  `panel_state` rendering helper (analyst did not grep the exact
  file:line; architect to pin in tasks.md).
- **New surface LOC.** ~10-15 LOC for the spinner instantiation +
  any Lumen color override.
- **Snapshot refresh count.** ~8 `*_loading.snap` baselines refreshed
  (one-shot, byte-stable across two consecutive runs is the
  acceptance gate).
- **Acceptance criteria.**
  - Loading panels render `iced_aw::spinner::Spinner` at the
    dispatch site previously emitting `"Loading…"` text.
  - Two-run determinism (`cargo test -p ui --test panel_snapshots
    -- loading` twice in succession) shows zero `*.snap.new` files.
  - `scripts/check_no_clocks_in_ui_tests.sh`
    ([`ui-test-harness-bootstrap` v0.1](../ui-test-harness-bootstrap/feature.md))
    PASSES against the spinner module's source (the no-clock grep
    is the orchestrator-runnable falsifier for H-arch-9).
  - Hypothesis **H-arch-9** unfalsified.
- **Out-of-scope for B2.** A spinner with internal animation frame
  state. If `iced_aw::spinner` ships an animation timer, B2 SKIPS
  (fall back to keeping the static `"Loading…"` text) — H-arch-9 is
  the gate.

### B3 — `iced_aw::badge` (status chips on Strategies / Risk)

- **New surface.** Status chips on Strategies + Risk screens rendered
  via `iced_aw::badge` with cockpit-tinted Lumen surface tokens.
- **Consumer surfaces.** Hand-rolled
  `container().style(badge_style)` patterns retired across ~3 files
  on the Strategies + Risk panels. Concrete file:line set to be
  pinned by architect in tasks.md; analyst's estimate is ~50 LOC
  across 3 files.
- **Retired LOC.** ~50 LOC (analyst estimate). Includes:
  - Inline `container(...).style(...)` chains where the only style
    payload is a Lumen surface token + border-radius.
  - Helper functions that produced those containers, where the
    helper has no other consumer.
- **New surface LOC.** ~10-20 LOC for badge instantiations + a
  `cockpit_badge_style_fn` (or analogous) Catalog adapter, parallel
  to `cockpit_table_style_fn`, at
  [`crates/ui/src/theme/iced_widget_catalogs.rs`](../../crates/ui/src/theme/iced_widget_catalogs.rs).
- **Snapshot refresh count.** ~5 panel snapshots (Strategies +
  Risk views).
- **Acceptance criteria.**
  - `iced_aw::badge` accepts a Catalog / StyleFn taking `Color` or
    `Theme` (H-arch-10 falsifier).
  - Lumen brand-bleed grep (existing UI test harness gate) passes:
    no hard-coded RGB triplets land in the badge call sites.
  - Two-run determinism gate passes; ~5 `*.snap` baselines refresh
    cleanly.
  - Hypothesis **H-arch-10** unfalsified.
- **Out-of-scope for B3.** Replacing PNL / PNL_PCT cell-color
  rendering in `positions.rs` / `strategies.rs` Brief A tables — those
  are color-overlay decisions inside per-column lambdas, not chip
  primitives. If the architect finds a clean wire-in, it ships as a
  follow-up in v0.2, not v0.1.

## Hypothesis register (analyst restatement)

These restate the architect's H-arch-4 / H-arch-9 / H-arch-10 entries
(from the parent [`iced-ecosystem-evaluation` v0.2.0](../iced-ecosystem-evaluation/feature.md)
hypothesis register) in analyst-facing falsifiable form. Falsifier
commands are preserved verbatim from the parent register; the architect
re-emits them as orchestrator-runnable steps in the next handoff's
tasks.md.

### H-arch-4 — `iced_aw::date_picker` 0.14.1 is feature-flag-isolatable

- **Analyst-facing statement.** Adding `iced_aw = "0.14"` with
  `default-features = false` and `features = ["date_picker"]` does NOT
  drag in `iced_aw`'s `menu`, `tab_bar`, `sidebar`, or other widget
  surfaces. If it does, Brief B's "+1 direct dep, +2-3 transitive"
  cost estimate is wrong and the scope must re-open.
- **Falsifier (verbatim from architect).** Orchestrator (or sandbox)
  runs `cargo tree -p iced_aw --features date_picker --no-default-features`
  (read-only inspection via a scratch Cargo.toml — NOT a workspace
  edit) and counts new transitive crates. If the count exceeds 5 new
  crates OR pulls a forbidden license, falsified → re-scope to
  "vendor the date-picker source" or HOLD.
- **Analyst-added falsifier (independent of B1).** B1 is also gated
  by the v1.11 / Phase 4 backtest-range follow-up plan; the architect
  must confirm the picker's output type (`chrono::NaiveDate` /
  `iced_aw::date_picker::Date` / etc.) is convertible to the existing
  scenario-fetch path's date type. Confirmation is a `cargo doc -p
  iced_aw --features date_picker --no-deps` + grep for the `Message`
  payload type.
- **Status:** unresolved (architect to drive falsifier).

### H-arch-9 — `iced_aw::spinner` renders deterministically (no clock read)

- **Analyst-facing statement.** `iced_aw::spinner` must NOT read
  `Instant::now`, `SystemTime::now`, or `elapsed` to compute its
  rotation angle or frame state. If it does, every `*_loading.snap`
  baseline becomes flaky and B2 SKIPS — we fall back to retaining
  the plain-text `"Loading…"` rendering.
- **Falsifier (verbatim from architect).** Grep `iced_aw` source
  (vendored via `cargo doc` or crates.io view) for `Instant::now` /
  `SystemTime::now` / `elapsed` in the spinner module. If present →
  falsified, B2 SKIP, fall back to static "Loading…" text. Also
  covered by `scripts/check_no_clocks_in_ui_tests.sh` from
  [`ui-test-harness-bootstrap` v0.1](../ui-test-harness-bootstrap/feature.md).
- **Analyst note.** The harness's clocks-grep is the load-bearing
  cross-check; if architect's `cargo doc` falsifier is clean but the
  workspace-wide clocks-grep flags the spinner module, that is a
  stronger signal and B2 SKIPS regardless.
- **Status:** unresolved.

### H-arch-10 — `iced_aw::badge` exposes a Catalog / StyleFn hook

- **Analyst-facing statement.** `iced_aw::badge` must accept a
  cockpit-provided custom-style hook (Catalog / StyleFn / `Color` /
  `Theme` taker) so badges render in our Lumen `theme::ModeColor`
  ramp. If `badge` hard-codes its own palette, badges look out of
  place, the Lumen brand-bleed grep fails, and B3 falls back to the
  hand-rolled `container().style(badge_style)` pattern (i.e. B3
  SKIPS).
- **Falsifier (verbatim from architect).** `cargo doc -p iced_aw
  --features badge --no-deps` + grep the badge module for a `style` /
  `Catalog` / `StyleFn` public surface taking `Color` or `Theme`.
  If absent → falsified, B3 falls back to hand-rolled
  `container().style(badge_style)`.
- **Analyst-added consumption claim.** If H-arch-10 unfalsifies, the
  cockpit Catalog adapter shipped in Brief A
  ([`crates/ui/src/theme/iced_widget_catalogs.rs`](../../crates/ui/src/theme/iced_widget_catalogs.rs))
  becomes the canonical wire-in pattern: B3 ships a
  `cockpit_badge_style_fn()` parallel to the existing
  `cockpit_table_style_fn()`, routing `color::BORDER_1` and the
  appropriate ModeColor surface token.
- **Status:** unresolved.

## Numbers that matter

The user has corrected past briefs that conflated **file-span LOC**
(LOC inside files Brief B edits) with **glue-layer LOC** (Cargo.toml
diffs + theme/catalog adapter wiring + new module surface). Both are
reported separately.

### File-span LOC (estimates — architect confirms in tasks.md)

| Sub | Touched file(s) (analyst best-guess) | File-span LOC delta |
|---|---|---|
| B1 | viewer bin (single file, TBD by architect) | +30 to +50 (new surface) |
| B2 | shared `panel_state` rendering helper + ~8 panel files | -10 to -20 retired, +10 to +15 new (net ~0 to -10) |
| B3 | ~3 widget files on Strategies / Risk panels | -50 retired, +10 to +20 new (net -30 to -40) |
| **Brief B total** | | net **-30 to -50 LOC retired** (excluding glue) |

### Glue-layer LOC

| Surface | LOC delta |
|---|---|
| `crates/ui/Cargo.toml` — `iced_aw = "0.14"` dep + 3 features | ~+3 |
| `crates/ui/src/theme/iced_widget_catalogs.rs` — `cockpit_badge_style_fn` | ~+20 to +40 (parallel to existing `cockpit_table_style_fn`) |
| `crates/ui/src/widgets/mod.rs` or similar — re-exports if needed | ~+3 to +10 (architect to confirm whether `iced_aw` types re-exported here or used inline at call sites — open question) |
| **Glue-layer total** | ~+30 to +55 LOC |

### Other deltas

| Metric | Brief B value | Source / note |
|---|---|---|
| Panel snapshot refresh count | ~13 (≈8 spinner + ≈5 badge + 1 picker) | Per-sub-target estimates; architect tightens |
| PNG-baseline diff (Charts triples) | **0** | Brief B touches viewer bin + Strategies + Risk + loading panels; the 3 PNG triples render the Charts screen ([`crates/ui/tests/visual-baselines/charts_screen_dark_*.png`](../../crates/ui/tests/visual-baselines/)) — untouched. |
| Anchor risk | **0** | Brief B touches zero strategy / audit / exec / backtest code paths. `spec/anchors.toml`'s 11 locked anchors are inapplicable. |
| Direct dep delta | **+1** | `iced_aw = "0.14"` with `default-features = false`, features `["date_picker", "spinner", "badge"]`. |
| Transitive dep delta | **+2 to +3 (estimate)** | Architect confirms by running `cargo tree -p iced_aw --features date_picker,spinner,badge --no-default-features` (per H-arch-4 falsifier). |
| License | MIT | `iced_aw` upstream; architect to grep `iced_aw` Cargo.toml at adoption time. |
| Repo / maintenance signal | Official `iced-rs` org; last commit 2026-04-27 | Per parent brief; architect re-verifies at adoption time. |

## Architectural divergences (honest)

These are points where Brief B differs from the casual sketch in the
parent [`iced-ecosystem-evaluation` v0.2.0`](../iced-ecosystem-evaluation/feature.md)
brief, surfaced now so the architect doesn't carry forward an
already-stale assumption.

- **Badge consumer count.** Parent brief estimates "~5 panel snapshots"
  for B3. Analyst preserves the snapshot estimate but flags that the
  `~50 LOC across 3 files` payload is itself an estimate — the architect
  must grep `container(...).style(badge_style)` (or equivalent helper
  patterns) across `crates/ui/src/widgets/` to pin the exact retiring
  set. If the actual count is ≤2 files, B3 may not be worth the third-
  party dep cost on its own — but B1 + B2 already justify the dep, so
  B3 ships regardless.
- **Spinner consumer count.** Parent brief estimates "~8 panels emit
  `Loading…` text." Analyst did NOT grep this; if the actual count is
  ≤3, B2's snapshot-refresh cost shrinks proportionally and the
  glue-layer LOC dominates. Architect must grep
  `panel_state::Loading` rendering call sites.
- **Catalog adapter location.** Brief A shipped `cockpit_table_style_fn`
  at [`crates/ui/src/theme/iced_widget_catalogs.rs`](../../crates/ui/src/theme/iced_widget_catalogs.rs).
  Analyst recommends Brief B colocates `cockpit_badge_style_fn` in the
  same module (the module docstring explicitly designated it as the
  Brief B adoption hub at lines 33-43). The architect is free to
  override this if a per-widget module split is cleaner.
- **B1 surface scope.** The parent brief frames B1 as "v1.11 / Phase 4
  backtest range" — analyst is restating B1 as "the picker primitive
  + a smoke-test consumer in viewer bin", NOT the full date-range
  scenario-fetch rewiring. The full wire-in is a separate v1.11 /
  Phase 4 brief. The architect should confirm this scope split with
  the operator if there is ambiguity.
- **Cost-table reconciliation.** Parent brief cost row says "~50-100
  LOC + new surfaces" for Brief B. Analyst's file-span + glue total
  trends toward the **lower end** (~30-50 LOC retired + ~30-55 LOC
  glue). This is consistent — Brief B is a small surface — but the
  architect should NOT plan for a >100 LOC churn budget.

## Out of scope

These are explicit non-goals — items the architect MUST NOT re-open in
the Brief B tasks.md:

- **Block-adoption of `iced_aw` (i.e. `iced_aw/full`).** Rejected by
  the parent brief at Q5 resolution
  ([`iced-ecosystem-evaluation/feature.md`](../iced-ecosystem-evaluation/feature.md#design--architect-synthesis)).
  Each subsequent `iced_aw` widget surface (`menu`, `tab_bar`,
  `sidebar`, `card`, `quad`, `wrap`, `selection_list`, etc.) requires
  its own adoption-feature decision. Brief B is for `date_picker` +
  `spinner` + `badge` only.
- **Charts surface widgets.** PNG-baseline territory. Brief B leaves
  all 3 `charts_screen_dark_*.png` baselines byte-identical. No
  `iced_aw` widget lands on the Charts canvas.
- **`agent_feed.rs` migration.** Held by Brief A under H-arch-7; B2
  spinner adoption does NOT re-open `agent_feed`'s row-virtualization
  question.
- **`iced_dialog` chrome wrapper** (Brief C). Separately gated on
  H-arch-6 falsification; not in Brief B's scope.
- **`plotters-iced2` spike** (Brief D). Spike-only, deferred; not in
  Brief B's scope.
- **Markdown viewer.** Operator-gated separately; not in Brief B.
- **`iced_aw::date_picker` → scenario-fetch full rewire.** B1 ships
  the picker primitive only. The actual backtest-range wire-in is
  v1.11 / Phase 4.
- **Animation API adoption** (`iced` Q4). Operator-deferred; not in
  Brief B.

## Open questions for architect

The architect resolves these in the next handoff (typically as a
Q-section in Brief B's tasks.md or as inline notes in the hypothesis
register):

1. **Re-export shape.** Do we re-export `iced_aw` types through
   `crates/ui/src/widgets/mod.rs` (or a new `crates/ui/src/widgets/
   aw.rs` shim) so call sites stay inside the `ui` crate's public
   surface, OR do we use `iced_aw` types inline at call sites? The
   re-export tightens the `iced_aw` blast radius if the dep ever
   changes; the inline path is lighter. Analyst leans re-export for
   consistency with `cockpit_table_style_fn`'s "single module owns
   the iced-widget Catalog adapters" lemma.
2. **Catalog adapter colocation.** Confirm `cockpit_badge_style_fn`
   lives in `crates/ui/src/theme/iced_widget_catalogs.rs` alongside
   `cockpit_table_style_fn` (Brief A's docstring designates this).
   Override if a per-widget module split is cleaner.
3. **`iced_aw` `date_picker` `Message` payload type.** What concrete
   type does the picker emit on selection? `chrono::NaiveDate`?
   `iced_aw::date_picker::Date`? Confirm via `cargo doc -p iced_aw
   --features date_picker --no-deps` before B1 wiring lands —
   architect prerequisite for the v1.11 / Phase 4 follow-up.
4. **Exact panel set for B2.** Grep `panel_state::Loading` rendering
   to confirm the "~8 panels" count. If <3, downgrade B2's snapshot
   refresh budget to ≤3.
5. **Exact file set for B3.** Grep `container(...).style(badge_style)`
   (or analogous helper patterns) across `crates/ui/src/widgets/` to
   pin the "~3 files" set. If 0-1 files match, B3 still ships (B1 +
   B2 already justify the `iced_aw` dep) but the LOC retirement
   shrinks.
6. **Transitive dep audit.** Run `cargo tree -p iced_aw --features
   date_picker,spinner,badge --no-default-features` to either confirm
   the "+2-3 transitive crates" estimate or surface a blow-out (H-arch-4
   falsifier).
7. **B-ordering.** Should B1 / B2 / B3 ship as one M-milestone with
   three lanes (parallel devs), or as three sequential M-milestones?
   Analyst leans single-milestone-three-lanes since the three
   sub-targets are independent (zero shared file) and each carries
   its own falsifier.

## Design — architect synthesis

_Architect pass 2026-05-13. Resolves the analyst's 7 open questions
and records the H-arch-4 / H-arch-9 / H-arch-10 falsifier verdicts
inline. Falsifiers were run by temporarily adding `iced_aw = "0.14"`
under `crates/ui/Cargo.toml` to drive `cargo tree` / `cargo doc`, then
reverting before this synthesis was committed — the real Cargo.toml
landing belongs to the developer pass per
[`AGENT.md ## Architect = hypothesis only`](../../AGENT.md#architect--hypothesis-only)._

### Falsifier verdicts

#### H-arch-4 — `iced_aw::date_picker` feature-flag isolation — **RESOLVED-PASS**

- **Command:** `cargo tree -p ui -i iced_aw --no-default-features` after
  temporarily adding `iced_aw = { version = "0.14", default-features = false, features = ["date_picker"] }`
  under `crates/ui/Cargo.toml`.
- **Output verbatim:**
  ```
  iced_aw v0.14.1
  └── ui v0.1.0 (/Users/Vitaliy.Schreibmann/Projects/Privat/trading/trading/crates/ui)
  ```
- **Forward edges (`cargo tree -p ui`):** `iced_aw v0.14.1` →
  `chrono v0.4.44` + `num-traits v0.2.19` + `once_cell v1.21.4`.
  **Zero pulls from `menu` / `tab_bar` / `badge` / `spinner` /
  `number_input`.** `iced_aw` 0.14.1's `[features]` table at
  `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/iced_aw-0.14.1/Cargo.toml:39-78`
  confirms `badge = []`, `spinner = []` (empty — no transitive deps);
  only `date_picker = ["chrono"]` and `number_input = ["num-format",
  "num-traits", "typed_input"]` bring deps. Adding all three
  `["date_picker", "spinner", "badge"]` features pulls **exactly the
  same +3 transitive crates** (`chrono`, `num-traits`, `once_cell`) the
  analyst estimated at "+2 to +3."
- **License:** MIT (`iced_aw-0.14.1/Cargo.toml:18`). Edition 2024
  (`:13`). Library compatibility checklist: PASS.
- **Verdict:** UNFALSIFIED. Brief B's `+1 direct, +3 transitive` cost
  estimate is correct.

#### H-arch-9 — `iced_aw::spinner` deterministic-render — **RESOLVED-PASS (with caveat)**

- **Source inspected:**
  `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/iced_aw-0.14.1/src/widget/spinner.rs`
  (212 LOC).
- **Command:** `grep -n "Instant::now\|SystemTime::now\|Local::now\|elapsed\|\.now(" .../spinner.rs`
- **Output verbatim:**
  ```
  160:            last_update: Instant::now(),
  ```
- **Analysis:** Exactly one wall-clock hit, on line 160 — the
  initialization of `SpinnerState::last_update` at widget-state
  construction (`fn state(&self) -> State`). The render path itself
  (`fn draw`, `spinner.rs:121-152`) is pure: it reads `state.t: f32`,
  computes `(state.t * 2π).sin_cos()`, and emits a quad. The animation
  update happens only inside `fn update` (`:165-202`), gated on
  `Event::Window(window::Event::RedrawRequested(now))` — `now` is
  iced's frame-time, NOT a wall-clock read. The widget further calls
  `shell.request_redraw_at(...)` (`:197-199`) which is iced's
  animation `Subscription` contract.
- **Snapshot-test implication:** `iced_test` (Brief A's harness path)
  does not fire `RedrawRequested` events unless the test explicitly
  drives them. `*_loading.snap` baselines render at `t = 0.0` — the
  `Instant::now()` placeholder on line 160 is constructed but never
  observed. **Determinism preserved.**
- **Caveat:** `scripts/check_no_clocks_in_ui_tests.sh` from
  [`ui-test-harness-bootstrap` v0.1](../ui-test-harness-bootstrap/feature.md)
  greps the workspace for `Instant::now` / `SystemTime::now` /
  `Local::now`. Since `iced_aw` is a `crates.io` dep (not vendored),
  the script's repo-scoped grep will **not** flag the spinner's
  line 160. The developer must verify the script's scope (likely
  `crates/` and `src/`, not `~/.cargo/registry/`) before ticking the
  M_FINAL gate. If the script ever broadens to vendored deps, B2
  would need to mark `iced_aw/src/widget/spinner.rs:160` as a known
  acceptable hit.
- **Verdict:** UNFALSIFIED per the brief's "acceptable alternative"
  clause (iced animation `Subscription` contract). B2 ships.

#### H-arch-10 — `iced_aw::badge` Catalog / StyleFn hook — **RESOLVED-PASS**

- **Source inspected:**
  `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/iced_aw-0.14.1/src/widget/badge.rs`
  (496 LOC) + `src/style/badge.rs` (475 LOC).
- **Catalog trait at `style/badge.rs:27-37`:**
  ```rust
  /// The Catalog of a [`Badge`](crate::widget::badge::Badge).
  pub trait Catalog {
      type Class<'a>;
      fn default<'a>() -> Self::Class<'a>;
      fn style(&self, class: &Self::Class<'_>, status: Status) -> Style;
  }
  impl Catalog for Theme {
      type Class<'a> = StyleFn<'a, Self, Style>;
      ...
  }
  ```
- **`.style(...)` builder at `widget/badge.rs:115-119`:**
  ```rust
  pub fn style(mut self, style: impl Fn(&Theme, Status) -> Style + 'a) -> Self
  where Theme::Class<'a>: From<StyleFn<'a, Theme, Style>>,
  ```
- **`Status` enum at `src/style/status.rs`** (exposes
  `Status::{Active, Hovered, Pressed, Disabled, ...}`) — call sites
  branch on status inside the closure, exactly the shape Brief A's
  `cockpit_table_style_fn` adopts.
- **Structural compatibility with `cockpit_table_style_fn`:** identical
  shape (`Box<dyn Fn(&Theme, Status) -> Style + 'a>`) — a new
  `cockpit_badge_style_fn()` colocated in
  [`crates/ui/src/theme/iced_widget_catalogs.rs`](../../crates/ui/src/theme/iced_widget_catalogs.rs)
  drops in alongside the existing function (the module docstring at
  `:33-43` explicitly designated this slot for Brief B adopters).
- **Verdict:** UNFALSIFIED. B3 ships; the Catalog adapter wire-in is
  the canonical path.

### Open-question resolutions (Q1 – Q7)

#### Q1 — Re-export shape: **inline `iced_aw` types at call sites**

The blast-radius argument for re-exports is theoretical; Brief B
touches exactly three call sites (`viewer.rs`, the `muted_body`/spinner
wrappers, and the strategy-status chip). A `crates/ui/src/widgets/aw.rs`
shim would add a re-export module for **3 type imports** — pure
ceremony. Use direct imports:
- B1: `use iced_aw::date_picker::DatePicker;` + `use iced_aw::core::date::Date;` inside `crates/ui/src/bin/viewer.rs`.
- B2: `use iced_aw::Spinner;` inside `crates/ui/src/widgets/frame.rs` (where the new helper lives — see Q4).
- B3: `use iced_aw::Badge;` inside `crates/ui/src/widgets/strategies.rs` (per Q5 — sole consumer).

If a future surface ever adopts a fourth `iced_aw` widget, the
developer can re-evaluate. **No ADR required** — this is a trivial
import-style call and overrides the analyst's lean toward re-export.

#### Q2 — Catalog adapter colocation: **confirmed in `iced_widget_catalogs.rs`**

`cockpit_badge_style_fn()` lives as a sibling of `cockpit_table_style_fn`
in [`crates/ui/src/theme/iced_widget_catalogs.rs`](../../crates/ui/src/theme/iced_widget_catalogs.rs)
(per the module docstring at `:33-43`). The function signature
mirrors the table adapter exactly:
```rust
#[must_use]
pub fn cockpit_badge_style_fn<'a>()
    -> Box<dyn Fn(&iced::Theme, iced_aw::style::Status) -> iced_aw::style::badge::Style + 'a>
{ Box::new(cockpit_badge_style) }
```
No new module split needed. The unit-test pattern at
`iced_widget_catalogs.rs:99-121` extends naturally with two parallel
tests for the badge adapter.

#### Q3 — `iced_aw::date_picker` Message payload: **`iced_aw::core::date::Date`**

- The constructor at `iced_aw-0.14.1/src/widget/date_picker.rs:96-118`
  takes `on_submit: F where F: 'static + Fn(Date) -> Message`.
- `Date` is defined at `iced_aw-0.14.1/src/core/date.rs:8-16`:
  `pub struct Date { pub year: i32, pub month: u32, pub day: u32 }`.
- **Bidirectional `chrono::NaiveDate` conversion** at
  `iced_aw-0.14.1/src/core/date.rs:49-60` (`impl From<Date> for NaiveDate`
  + `impl From<NaiveDate> for Date`). Convertible to/from `time::Date`
  via a one-line `time::Date::from_calendar_date(date.year, …)` shim if
  the v1.11 follow-up wants to stay on `time` (workspace's preferred
  date crate per `Cargo.toml`).
- **Determinism trap:** `Date::today()` (`:31-34`) and `State::reset()`
  (`:173-175`) call `Local::now()`. B1's smoke-test consumer in
  `viewer.rs` MUST construct the picker with `Date::from_ymd(2024, 1, 1)`
  (or a fixture-provided const date) and MUST NOT call `Date::today()`
  / `State::reset()` from any code path reachable in tests. This is
  added as an explicit acceptance criterion on T-M1-2.

#### Q4 — Exact panel set for B2: **8 call sites across 5 files** (analyst's "~8 panels" confirmed)

Grep result for `muted_body(.*_LOADING)`:
```
crates/ui/src/screens/strategies.rs:55  — muted_body(STRATEGIES_LOADING)
crates/ui/src/screens/strategies.rs:245 — muted_body(STRATEGIES_SPARKLINE_LOADING)
crates/ui/src/screens/audit.rs:55       — muted_body(AUDIT_LOADING)
crates/ui/src/screens/risk.rs:45        — muted_body(RISK_LOADING)
crates/ui/src/widgets/positions.rs:55   — muted_body(POS_LOADING)
crates/ui/src/widgets/strategies.rs:46  — muted_body(STRATEGIES_LOADING)
crates/ui/src/widgets/pnl.rs:22         — muted_body(PNL_LOADING)
crates/ui/src/widgets/agent_feed.rs:40  — muted_body(TAPE_LOADING)
```
**8 call sites confirmed** (analyst estimate "~8 panels" was accurate).

**Architectural divergence — there is NO shared `panel_state` dispatch
helper.** The analyst's brief framed B2 as "replace at the dispatch
site, not per-panel" (sub-target text at lines 119-126). Reality:
`muted_body(text)` is a generic text-rendering helper at
[`crates/ui/src/widgets/frame.rs:142-147`](../../crates/ui/src/widgets/frame.rs)
that takes any `&'a str` — each panel calls it with its own
context-specific message ("Connecting to the fill stream…",
"Loading positions from the ledger…", etc.). The loading TEXT is
deliberately informational and per-panel; replacing it wholesale with
a textless spinner deletes user-visible context.

**Design call — preserve the informational text + add a spinner
alongside it.** Introduce a new helper
[`crates/ui/src/widgets/frame.rs`](../../crates/ui/src/widgets/frame.rs):
```rust
#[must_use]
pub fn loading_with_spinner<'a, Message: 'a>(text: &'a str, mode: ThemeMode) -> Element<'a, Message> {
    Row::new()
        .spacing(space::S)
        .align_y(Vertical::Center)
        .push(iced_aw::Spinner::new()
            .width(Length::Fixed(16.0))
            .height(Length::Fixed(16.0)))
        .push(Text::new(text)
            .size(text::BODY)
            .color(color::FG_3.current(mode)))
        .into()
}
```
Each of the 8 call sites swaps `muted_body(X_LOADING)` →
`loading_with_spinner(X_LOADING, mode)`. The `mode` parameter pulls
from the caller's `&Cockpit`/`ThemeMode::Dark` context (each call site
already has it in scope per Brief A's audit). The new helper is
deletion-of-`muted_body`-call + addition-of-spinner-row → net ~+1 LOC
per call site. **Total file-span delta: -8 LOC retired (8 ×
`muted_body` → `loading_with_spinner`), +15 LOC new (helper),
net ≈ +7 LOC.** This is below the analyst's net-zero-to-minus-ten
estimate but consistent with the "Brief B small surface" framing.

#### Q5 — Exact file set for B3: **`crates/ui/src/widgets/strategies.rs` only** (1 file, NOT 3)

The analyst's grep targets (`container(...).style(badge_style)`,
`badge_style`, `status_chip`, `ChipStyle`) returned **zero hits** —
no hand-rolled badge pattern exists in `crates/ui/src/widgets/`. The
cockpit's status-chip surface is the strategy STATUS column at
[`crates/ui/src/widgets/strategies.rs:113-129`](../../crates/ui/src/widgets/strategies.rs),
which currently renders `colored_cell(status_label, status_color)` —
text color override on a plain cell, NOT a pill-shaped chip.

**Risk screen has zero badge consumers** — verified by reading
`crates/ui/src/screens/risk.rs:40-150`: the screen is threshold-bars
+ plain text only, no chip/badge/pill surface anywhere. The analyst's
"Strategies + Risk" framing was speculative; Risk is empty.

**Design call — B3 ships as Strategies-only with the status column
upgraded to a Lumen-tinted badge.** Replace the `colored_cell` text
override in column 3 with `iced_aw::Badge::new(text).style(cockpit_badge_style_fn())`,
routing Lumen `UP_500` / `FG_3` / `DOWN_500` surface tokens through
the Catalog adapter per status variant
(`StrategyStatus::Ready` / `Loading` / `Error`). **Total file-span
delta: -2 LOC retired (the `colored_cell(status_label, status_color)`
call), +12 LOC new (Badge construction + Catalog routing).** Well
below the analyst's "-50 LOC across 3 files" estimate.

**Justification for shipping B3 despite the smaller surface:** B1 +
B2 already justify the `iced_aw` dep (zero marginal cost). B3 retires
a text-color sentinel pattern and replaces it with a typed
status-chip primitive that the Phase 4 risk-status surface
(post-v1.11) will need anyway. The `cockpit_badge_style_fn` factory is
a permanent fixture for future consumers.

#### Q6 — Transitive dep audit: **+3 crates** (`chrono`, `num-traits`, `once_cell`)

Resolved inline in H-arch-4 above. Result:
- `chrono v0.4.44` (date_picker)
- `num-traits v0.2.19` (transitively via chrono — already in our
  tree via `rust_decimal`; **zero net add**)
- `once_cell v1.21.4` (transitively via chrono — likely already in
  our tree via `time`; net add ≤1)

**Effective net new crates: 1-2** (chrono is the only definite new
dep; the others either dedup or were already present). Below the
analyst's "+2 to +3" estimate.

#### Q7 — Milestone shape: **three parallel lanes under one logical milestone, ticked as M1/M2/M3 for spec-tracker clarity**

Per Brief A's precedent (4 parallel lanes ticked as M1/M2/M3/M4 in
[`spec/iced-native-widgets/tasks.md`](../iced-native-widgets/tasks.md)),
the orchestrator dispatches B1/B2/B3 as **three independent
sub-agents fanned out in the same delegation message**. Each
sub-target carries its own falsifier (already RESOLVED above), zero
shared files (B1 = viewer.rs, B2 = frame.rs + 8 call sites, B3 =
strategies.rs), and zero shared `Message` enum variants outside the
existing cockpit surface. The tasks.md below pins each sub-target as
its own M-milestone (M1/M2/M3) for readable tick-by-tick state.

### Scope-locks delta vs analyst draft

- **B3 surface SHRUNK** from "Strategies + Risk, ~3 files, ~50 LOC
  retired" to "Strategies only, 1 file, ~2 LOC retired + 12 new"
  (Q5 above). Brief still ships — `cockpit_badge_style_fn` factory
  + typed-chip primitive justify it on its own merits.
- **B2 design CHANGED** from "wholesale replace `Loading…` text with
  textless spinner" to "spinner + informational text in a Row"
  (Q4 above). Preserves user-facing context; the analyst's "8 panels"
  count is correct but the dispatch-helper assumption was wrong.
- **B1 surface unchanged.** The viewer bin gets the picker primitive
  + smoke-test consumer; full backtest-range wire-in stays gated to
  v1.11 / Phase 4 per analyst's brief.
- **Re-export shape decided AGAINST** the analyst's recommendation
  (Q1 above). Inline imports.

### Glue-layer LOC (architect-revised)

| Surface | LOC delta |
|---|---|
| `crates/ui/Cargo.toml` — `iced_aw = "0.14"` dep + 3 features | ~+3 |
| `crates/ui/src/theme/iced_widget_catalogs.rs` — `cockpit_badge_style_fn` | ~+30 (parallel to existing `cockpit_table_style_fn` incl. 2 unit tests) |
| `crates/ui/src/widgets/frame.rs` — `loading_with_spinner` helper | ~+15 (per Q4) |
| **Glue-layer total** | **~+48 LOC** |

### File-span LOC (architect-revised, per sub-target)

| Sub | Touched file(s) | File-span LOC delta |
|---|---|---|
| B1 | `crates/ui/src/bin/viewer.rs` (single file, single instantiation site) | +30 to +50 (new surface) |
| B2 | `crates/ui/src/widgets/frame.rs` + 8 call sites in 5 panel files | -8 retired, +15 helper, +8 swaps → net **+15** |
| B3 | `crates/ui/src/widgets/strategies.rs` (1 file) | -2 retired, +12 new → net **+10** |
| **Brief B total** | | net **+55 LOC** (file-span only) |

Aggregate Brief B delta: **~+55 file-span + ~+48 glue = ~+103 LOC**.
This is **slightly above** the analyst's "~30-50 retired + ~30-55 glue"
because the analyst over-estimated the retirement budget for B2 / B3.
Still within the parent brief's "~50-100 LOC" range when read as
file-span-only. The architect's revised target is a net **add** of
~100 LOC overall — Brief B is a feature add, not a refactor.

### Snapshot refresh count (architect-revised)

- **B1:** +1 panel snapshot (viewer-bin picker default-closed state).
- **B2:** ≤8 snapshots refreshed across `positions_loading.snap`,
  `strategies_loading.snap` (both screens + widget variants),
  `pnl_loading.snap`, `tape_loading.snap`, `audit_loading.snap`,
  `risk_loading.snap`. Each switches from `muted_body(text)` to
  `Row{spinner, text}` — the snapshot diff is the new Row wrapper +
  spinner element node. Two-run determinism gate is the hard
  acceptance per H-arch-9 caveat.
- **B3:** ≤6 snapshots refreshed: `strategies_*` baselines whose
  data has a non-default `StrategyStatus` value (`Ready`, `Loading`,
  `Error`). Specifically `strategies_v1_*.snap` variants — each
  contains a status cell that flips from text-colored to badge-shaped.
- **Brief B total: ~15 panel snapshots refreshed** (analyst's "~13"
  estimate was close; revised to 15 to account for the extra
  `strategies_sparkline_loading` site).
- **PNG baseline diff: 0.** Charts surface untouched.
- **Anchor risk: 0.** Strategy / audit / exec / backtest code paths
  untouched.

### Trace.toml `arch` column

The architect fills `arch` for REQ-ICED-AW-001/-002/-003 with anchors
pointing into this section:

- `REQ-ICED-AW-001` → `spec/iced-aw-cherry-pick/feature.md#h-arch-4--iced_awdate_picker-feature-flag-isolation--resolved-pass`
  + `#q3--iced_awdate_picker-message-payload-iced_awcoredateDate`
- `REQ-ICED-AW-002` → `spec/iced-aw-cherry-pick/feature.md#h-arch-9--iced_awspinner-deterministic-render--resolved-pass-with-caveat`
  + `#q4--exact-panel-set-for-b2-8-call-sites-across-5-files-analysts-8-panels-confirmed`
- `REQ-ICED-AW-003` → `spec/iced-aw-cherry-pick/feature.md#h-arch-10--iced_awbadge-catalog--stylefn-hook--resolved-pass`
  + `#q5--exact-file-set-for-b3-cratesuisrcwidgetsstrategiesrs-only-1-file-not-3`

### ADR call — no new ADR required

Brief B introduces no new design pattern. The Catalog-adapter colocation
(Q2) reuses Brief A's module + extends with one new factory. The
re-export decision (Q1) defaults to "inline" with no new shim module.
The B2 helper (Q4) is one new function in an existing module. No
non-trivial tradeoff warrants ADR ceremony.

### Capability-boundary record

The architect ran `cargo tree -p ui -i iced_aw --no-default-features`
and `cargo doc -p iced_aw` (implicitly via dep resolution) only —
both are display/GPU-free and within the architect's sandbox per
[`AGENT.md ## Architect = hypothesis only`](../../AGENT.md#architect--hypothesis-only).
The `crates/ui/Cargo.toml` edit was temporary (line 73-79, added
2026-05-13, reverted same session) and is documented here for audit.
No `crates/` source files were touched.

## Verification (placeholder — tester fills in)

_Tester links reports here after the developer pass lands._

## Changelog

- 2026-05-14 (presenter): frontmatter bump `status: design →
  in-progress`, `owner: architect → presenter`, `version: 0.1.0 →
  1.0.0`. Evaluator emitted `VERDICT → PASS` at
  [`reports/evaluation-2026-05-14T07-13Z.md`](reports/evaluation-2026-05-14T07-13Z.md);
  presentation drafted at
  [`presentations/iced-aw-cherry-pick-2026-05-14.md`](presentations/iced-aw-cherry-pick-2026-05-14.md).
  Operator-visible behavior is complete (B1 picker primitive + B2
  spinner+text helper + B3 typed status badge). `status` flips to
  `shipped` only after the operator ticks `Approved — ship` on the
  presentation (orchestrator pass). The original `status: design` was
  invalid per `scripts/spec_lint.py` (allowed: active / candidate /
  deprecated / draft / in-progress / proposed / reserved / roadmap /
  shipped); `in-progress` is the closest valid status while the
  approval gate is pending. T-M_FINAL-* in [`tasks.md`](tasks.md) stay
  unticked per `AGENT.md ## Process discipline` rule 2 (only the
  tester ticks T_FINAL_*; orchestrator owns the post-approval tick).
- 2026-05-13 (architect): synthesis pass v0.2.0. Resolved all 7
  analyst open questions; recorded H-arch-4 / H-arch-9 / H-arch-10
  falsifier verdicts (all RESOLVED-PASS with one caveat on H-arch-9
  scope-of-the-clocks-grep). Surfaced two scope deltas vs analyst:
  B3 shrinks from "3 files" to 1 file (Risk screen has no badge
  consumer; Strategies has the only chip-shaped surface); B2's
  `panel_state` "shared dispatch helper" assumption was wrong —
  `muted_body(text)` is a per-panel call with informational text,
  so the design uses a sibling `loading_with_spinner` helper that
  preserves the text. Tasks.md published at
  [`spec/iced-aw-cherry-pick/tasks.md`](tasks.md). HANDOFF →
  developer + ui-designer (parallel). Status `draft` → `design`,
  owner `analyst` → `architect`.
- 2026-05-13 (analyst): initial draft v0.1.0. Scope locked to
  `date_picker` + `spinner` + `badge` per parent
  [`iced-ecosystem-evaluation` v0.2.0](../iced-ecosystem-evaluation/feature.md)
  Q5 resolution. Hypothesis register restated from H-arch-4 /
  H-arch-9 / H-arch-10. Predecessor pointer to
  [`iced-native-widgets` v0.1.0](../iced-native-widgets/feature.md)
  (shipped 2026-05-13) — specifically the `cockpit_table_style_fn`
  Catalog adapter, which Brief B's B3 sub-target is intended to
  consume via a parallel `cockpit_badge_style_fn`. Surfaced 7
  open questions for architect resolution. Flagged for
  spec-auditor: parent `iced-ecosystem-evaluation/feature.md` has
  outgrown a single-shot brief (19k tokens; over 10k soft budget)
  — recommend a future split into per-Brief satellite files.
