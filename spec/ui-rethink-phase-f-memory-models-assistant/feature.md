---
slug: ui-rethink-phase-f-memory-models-assistant
status: shipped
owner: operator
updated: 2026-05-20
version: 0.1.0
predecessor: ui-rethink-phase-e-compare v0.1.0
---

# UI rethink Phase F — Memory + Models + Phase-6 Assistant slot (J7 + J8 + Lumen Phase 6)

> Sixth and final concrete feature carved out of
> [`spec/dev-notes/ui-rethink-2026-05-17.md`](../dev-notes/archive/2026-Q2/ui-rethink-2026-05-17.md).
> Dev-note §6 Phase F (lines 1098-1112) is the **scope source-of-truth**;
> this brief is the **implementation contract**. Predecessor:
> [`ui-rethink-phase-e-compare v0.1.0`](../ui-rethink-phase-e-compare/feature.md)
> shipped 2026-05-20. The Phase A/C reservations are already in place
> for all three landed surfaces — Phase F wires the bodies:
>
> - **Memory** sidebar entry — `SIDEBAR_GROUPS_PHASE_C[1][1]` Library
>   zone (`crates/ui/src/theme.rs:745`); routed today via
>   `placeholder::view(strings::MEMORY_PLACEHOLDER, mode)` at
>   `crates/ui/src/shell.rs:98`.
> - **Models** sidebar entry — `SIDEBAR_GROUPS_PHASE_C[1][2]` Library
>   zone (`crates/ui/src/theme.rs:746`); routed today via
>   `placeholder::view(strings::MODELS_PLACEHOLDER, mode)` at
>   `crates/ui/src/shell.rs:99`.
> - **Phase-6 Assistant slot** — right-rail column-track reservation
>   wired at `crates/ui/src/shell.rs:47-49` with width pinned to
>   `RIGHT_RAIL_WIDTH_PX = 0.0` at `crates/ui/src/theme.rs:643`
>   ("Phase 2 — right-rail Phase 6 Assistant slot reservation … shell
>   renders this column with `Length::Fixed(0.0)` until the v2-LLM
>   Assistant ships in Phase 6").
>
> Phase F is **the final phase** of the UI rethink. Per dev-note §6
> line 1134, "No cliffs at C, E, F — each phase is independently
> shippable and independently reversible."

## Why

Two operator job-stories remain unaddressed by the cockpit after Phase
A-E shipped, plus the deferred Lumen Phase 6 slot becomes eligible
now that the v2 LLM strategy has landed:

1. **J7 — "Inspect the reflection memory"** (dev-note §J7, lines
   561-595). _"What did the agent learn last session? What lessons
   will it retrieve for tomorrow's first decision?"_ The reflection-
   memory crate has been writing `lesson_cards` to disk since
   `reflection-memory v0.1.0` (multiple closed-trade rows are already
   populating; the writer pipeline is live), but the operator has no
   way to see them inside the cockpit today. The locked moat is
   invisible. Phase F surfaces it.

2. **J8 — "Inspect a model version"** (dev-note §J8, lines 596-637).
   _"v2.5 TCN, v2.5a PatchTST, v2.5b Transformer, v2.6 bake-off. The
   operator wants to know: which checkpoint is currently serving?
   When was it trained? What was the val loss? What's the sigma_train?
   Is the forecast quality drifting?"_ As of 2026-05-20 the v2.5 TCN
   `BS-1` + `BS-2` checkpoints are on disk
   (`crates/forecast/checkpoints/anchors/tcn-bs1-*.{safetensors,metadata.json}`
   + `…-bs2-*.{safetensors,metadata.json}` — confirmed present) and the
   `forecast-distribution-bs1-realdata` / `…-bs2-realdata` anchors are
   locked in `spec/anchors.toml:156-161`. The operator currently has
   to grep the filesystem; Phase F surfaces the registry.

3. **Phase-6 Assistant slot (Lumen Phase 6)** — the right-rail column
   track has been reserved since Phase 2 (2026-04-xx); the wake
   condition was "v2 LLM strategy ships". **`v2-llm-strategy v2.0.0`
   shipped 2026-05-13** (backlog.md:1207-1213; operator-approved at
   `spec/v2-llm-strategy/presentations/v2-llm-strategy-2026-05-13.md`).
   The wake condition is **met**. Phase F is the eligible point to
   light the slot — but the operator may prefer a stub-only v0.1.0
   wake (see Q4 below) rather than a full text-stream wire at this
   pass, since the v2 LLM strategy ships as a research / pipeline
   surface, not yet a chat-shaped read surface.

The dev-note's §6 ordering puts Phase F last because **anchor risk is
zero by construction** (purely additive UI surface; no anchored-
renderer touch; no migration; no audit-ledger writer) and because the
three deliverables share a "wake reserved surfaces" character:
sidebar entries that already exist + a right-rail slot that already
exists — Phase F fills the bodies. The dev-note's framing at line
1110, _"Final sweep — anything missing?"_, makes Phase F's presenter
review the operator's last chance to surface a gap before the rethink
closes.

Key design tensions Phase F resolves:

- **Data-shape honesty in the Memory screen.** The reflection-memory
  `ReflectionStore` trait at `crates/reflection/src/store/mod.rs:27-35`
  exposes only `upsert / top_k / count` — there is no `list_all` or
  `list_recent_n` method. Phase F needs to either (a) add a read API
  to the trait (additive, low-risk) or (b) read directly from
  `SqliteReflectionStore`'s underlying table via a new `query`
  method. K1 below.
- **Models screen gating on checkpoint presence** — per the dev-note
  Phase F bullet at line 1105, the Models screen body is "gated on
  v2.5 landing a checkpoint per Q5". At 2026-05-20 both `BS-1` +
  `BS-2` checkpoints exist on disk; the screen has a real payload to
  render. Q5 from the original dev-note investigation (resolved
  2026-05-17 to "multi-source, all on disk already" per dev-note
  lines 1172-1177) is closed; this brief inherits the resolution.
- **Assistant slot v0 behavior** — Q4 below surfaces three options
  (stub / minimal-wire / defer) because the v2 LLM ship is **2 weeks
  old** at this pass and the operator may prefer to stabilise the
  v2 LLM surface itself before adding a UI-side chat-shaped
  consumer.
- **No cliffs** — per dev-note §6 line 1134, Phase F is independently
  shippable. The three deliverables are themselves independently
  shippable inside Phase F (Memory ↔ Models ↔ Assistant slot have
  zero coupling). The presenter sweep gates the operator's "anything
  missing?" review.

## Requirements

### R1 — `screens::memory` (J7 — reflection memory)

**R1.1** — New file `crates/ui/src/screens/memory.rs` exposing
`pub fn view(model: &Cockpit, mode: ThemeMode) -> Element<'_>`.
Replaces the current
`placeholder::view(strings::MEMORY_PLACEHOLDER, mode)`
body wired at `crates/ui/src/shell.rs:98`.

**R1.2** — `screens::memory::view` body composition (top to bottom),
following dev-note §J7 lines 567-580:
1. **Toolbar row** — mode toggle (Cards mode default per Q1a / dev-
   note §J7 line 571) + optional filter chip (per-strategy or
   per-symbol — defer to architect M-T1 per Q1).
2. **The cards list** — vertical scroll of one card per `LessonCard`
   row from the reflection-memory store. Each card per dev-note
   §J7 lines 571-575:
   - Trade context line: `<symbol_or_pair> · <closed_at> ·
     <outcome_class>`
   - Lesson body (markdown rendered; v0.1.0 the lesson body is
     deterministic per `reflection-memory v0.1.0` Q1=Option A —
     no LLM enrichment yet; renders as plain text)
   - Retrieval-relevance score (when card was retrieved by
     `retrieve_top_k`)
   - "Was this used recently?" stamp — count of decisions in last N
     days that retrieved this card (audit-ledger query; defer to
     architect M-T1 — Q6 below surfaces the cross-link defaults)
   - Right-aligned chevron → opens the source-trade detail (Q5
     below — drawer-vs-route)
3. **(Reserved for v0.2.0)** Cluster mode toggle — the weekly-
   distilled cluster view per `reflection-memory/feature.md` R8.
   Distillation is deferred per `crates/reflection/src/lib.rs:21-24`
   ("Q5 — periodic distillation deferred … lands in a follow-up
   brief `reflection-memory-distillation`"). v0.1.0 renders the
   Cluster toggle as `disabled` with a tooltip "Cluster view ships
   when distillation lands"; clicking it is a no-op.

**R1.3** — `Cockpit::current_screen == Screen::Memory` routes to
`screens::memory::view` (replaces the Phase A placeholder route in
`shell.rs:98`).

**R1.4** — Default state when reflection-memory has zero rows (cold
boot, no closed trades yet, no lesson cards) — render the
`widgets::placeholder` empty-state card with copy "No memory entries
yet. Memory populates as strategies close trades." (matches the
empty-state precedent from Phase A/C placeholders).

### R2 — `screens::models` (J8 — model versions)

**R2.1** — New file `crates/ui/src/screens/models.rs` exposing
`pub fn view(model: &Cockpit, mode: ThemeMode) -> Element<'_>`.
Replaces the current
`placeholder::view(strings::MODELS_PLACEHOLDER, mode)`
body wired at `crates/ui/src/shell.rs:99`.

**R2.2** — `screens::models::view` body composition (top to bottom),
following dev-note §J8 lines 603-637:
1. **Toolbar row** — family filter chips (`TCN` / `PatchTST` /
   `Transformer`) + status filter (`serving` / `staged` / `archived`)
   — v0.1.0 wires the chips with TCN-only enabled (the only family
   on disk at ship); other family chips render disabled with tooltip
   "Family ships in v2.5a / v2.5b".
2. **The model list** — one row per checkpoint discovered under
   `crates/forecast/checkpoints/anchors/*.metadata.json`. Each row
   per dev-note §J8 lines 605-614:
   - Model id (e.g. `tcn-bs1-d1c3696d…`)
   - Family (`TCN`)
   - Training data span (from `metadata.json`)
   - Val loss + train loss (from `metadata.json`)
   - `sigma_train` (the calibration constant)
   - Checkpoint SHA (truncated to 8 chars + tooltip with full SHA)
   - File size (`safetensors` byte count)
   - **Status pill** — `serving` / `staged` / `archived` (Q2 sub-
     decision: how is "serving" determined? — `serving` =
     mentioned by name in any `config/strategies/*.toml` running
     strategy at boot; v0.1.0 default = "all checkpoints on disk
     render as `staged`" with a tooltip; architect M-T1 to refine.)
   - **Forecast-quality sparkline** — last-N-bars residuals; **defer
     to v0.2.0** if the residual series is not on disk at v0.1.0
     ship time. Cell renders a placeholder if data is absent (per
     K3 below).
3. **(Reserved for v0.2.0)** Per-row detail panel — calibration plot,
   full metadata, audit-ledger consumption query, promote/archive/
   unload actions (destructive, gated). Phase F v0.1.0 is **read-
   only** (per anchor-risk-zero contract); destructive actions are a
   follow-up brief `models-screen-write-ops`.

**R2.3** — `Cockpit::current_screen == Screen::Models` routes to
`screens::models::view` (replaces the Phase A placeholder route in
`shell.rs:99`).

**R2.4** — Default state when no checkpoints are present on disk
(cold boot of a fresh checkout without `forecast/checkpoints/`) —
per Q3 below, **analyst-recommends (a)** honest placeholder: render
`widgets::placeholder` with copy "No models loaded yet. See
`spec/v25-tcn-overlay/feature.md` for how to train v2.5.0 TCN
checkpoints." Sidebar entry stays visible (not greyed out — Q3b
rejected because hiding the entry creates IA churn for cold-checkout
operators).

### R3 — Right-rail Assistant slot (Lumen Phase 6)

**R3.1** — `Cockpit::assistant_state: AssistantState` field added at
`state.rs:~880` (sibling of Phase E's `compare_screen_state` and
Phase D's `trail_screen_state`). `AssistantState` is a new struct in
a new module `crates/ui/src/assistant/state.rs`:
```rust
pub struct AssistantState {
    pub is_open: bool,
    pub mode: AssistantMode,  // see Q4
    pub messages: Vec<AssistantMessage>,  // populated only in modes b
}
```

**R3.2** — Right-rail slot wake — per Q4 (operator-decide), the
v0.1.0 behavior is one of:
- **(a) Stub-only** — `RIGHT_RAIL_WIDTH_PX` flips from `0.0` to a
  Lumen-aligned panel width (e.g. `320.0`); body renders a Tier 1
  placeholder card with copy "Assistant offline. v2 LLM wiring
  lands in v0.2.0." A toggle button in the status bar or chrome
  opens/collapses the slot. **Analyst-recommended default** —
  surfaces the slot honestly without committing to a chat-shaped
  consumer in the same pass.
- **(b) Minimal text-stream wire** — slot renders an input box +
  output box wired to `crates/llm::AnthropicProvider` via a new
  `AssistantMessage` message variant + the v2 LLM `LlmConfig`
  loader. Requires `tracing` + `tokio` plumbing across the
  iced/llm boundary; ~1 week of additional scope.
- **(c) Defer Assistant slot entirely** — Phase F ships only
  Memory + Models; the right-rail stays at `Length::Fixed(0.0)`;
  a follow-up brief `ui-rethink-phase-f-assistant-wire` lands
  separately. Anchor risk stays zero.

**R3.3** — Right-rail toggle affordance — for Q4=(a) or (b), a
collapse/expand button is placed in the status bar (right-aligned,
hairline-bordered) per the Lumen Phase 6 sketch at
[`spec/design/project/ui_kits/desktop/Assistant.jsx`](../archive/design-prototypes-2026-Q2.tar.gz).
Clicking flips `AssistantState::is_open`; when `false` the slot
collapses back to `RIGHT_RAIL_WIDTH_PX = 0.0`.

**R3.4** — `RIGHT_RAIL_WIDTH_PX` semantics — at v0.1.0 the constant
becomes a function of `AssistantState::is_open`:
```rust
fn right_rail_width(state: &AssistantState) -> f32 {
    if state.is_open { RIGHT_RAIL_OPEN_WIDTH_PX } else { 0.0 }
}
```
The existing `RIGHT_RAIL_WIDTH_PX = 0.0` constant is preserved as
the closed-state default; a new `RIGHT_RAIL_OPEN_WIDTH_PX` constant
is introduced. (For Q4=(c) this whole R3.4 block is dropped.)

### R4 — State plumbing

**R4.1** — `Cockpit::memory_screen_state: MemoryScreenState` field
added at `state.rs:~880` (sibling of `compare_screen_state`).
`MemoryScreenState` is a new struct in a new module
`crates/ui/src/memory/state.rs`:
```rust
pub struct MemoryScreenState {
    pub mode: MemoryViewMode,  // Cards (default) | Cluster (disabled)
    pub filter: Option<MemoryFilter>,  // per-strategy or per-symbol
    pub cache: Vec<LessonCard>,  // populated by cold-boot read
    pub last_indexed: Option<chrono::DateTime<Utc>>,
}
```

**R4.2** — `Cockpit::models_screen_state: ModelsScreenState` field
added at `state.rs:~880` (sibling of `memory_screen_state`).
`ModelsScreenState` is a new struct in a new module
`crates/ui/src/models/state.rs`:
```rust
pub struct ModelsScreenState {
    pub family_filter: Vec<ModelFamily>,  // [TCN] default
    pub status_filter: Vec<ModelStatus>,   // [Serving, Staged] default
    pub checkpoints: Vec<CheckpointMeta>,  // populated by cold-boot
    pub last_indexed: Option<chrono::DateTime<Utc>>,
}
```

**R4.3** — `Cockpit::assistant_state: AssistantState` field per
R3.1 (gated on Q4 ≠ (c)).

**R4.4** — Default values — all three screen states default to
empty/closed; cold-boot population happens on first screen-open
(Memory + Models) and on Assistant toggle (Assistant slot).

### R5 — Read paths (cold-boot population)

**R5.1** — **Memory cold-boot read.** New module
`crates/ui/src/memory/store_read.rs`. Exposes
`pub async fn list_recent_lesson_cards(store: &dyn ReflectionStore,
limit: usize) -> Result<Vec<LessonCard>, ReflectionStoreError>`.
This is a **new public API on the read path**.
- **K1 below tracks** the choice between (a) extending the
  `ReflectionStore` trait with a `list_recent` method (additive on
  the trait; touches every impl — currently just
  `SqliteReflectionStore`) versus (b) writing the SQL query
  directly inside `memory/store_read.rs` (one-call site; bypasses
  the trait). **Analyst recommends (b)** for v0.1.0 to keep the
  trait surface minimal; v0.2.0 lifts to (a) if a second consumer
  appears.

**R5.2** — **Models cold-boot read.** New module
`crates/ui/src/models/registry_read.rs`. Exposes
`pub fn discover_checkpoints(checkpoint_dir: &Path)
-> Vec<CheckpointMeta>`. Discovery walks
`crates/forecast/checkpoints/anchors/*.metadata.json`, parses each
JSON file, and builds `CheckpointMeta` from the metadata payload.
No new external deps (json parsing via `serde_json` which is
already in the workspace per `crates/forecast`).

**R5.3** — **Cache invalidation** — at v0.1.0 both Memory + Models
caches are **cold-boot-only** (mirrors Phase E R3.5). Operator
restarts the cockpit to see newly-written cards / newly-trained
checkpoints. v0.2.0 candidate: subscription bridges (Memory →
reflection-store writer event; Models → `crates/forecast` checkpoint-
write event).

### R6 — Cross-links (intra-cockpit navigation)

**R6.1** — **Memory → Trail.** Per dev-note §J7 line 580 ("Each card
has a chevron → Trail view for the trade that produced it"), the
Memory card's right-aligned chevron emits
`Message::OpenTrailFor(audit_id)` — the existing Phase D message
variant — passing the `close_transaction_id` from
`LessonCard::trade_context`. R6 mirrors Phase D's chevron-from-strategies
precedent (`crates/ui/src/screens/strategies.rs`).

**R6.2** — **Trail → Memory** (reverse cross-link). For each Trail
node that has an associated `lesson_card` (the audit-ledger
correlation column is `close_transaction_id`), the Trail drawer
gains a "View memory entry" link. This is **opt-in per Q6**
(operator-decide):
- (a) Trail (Phase D) cells link into Memory entries when relevant
  — touches Phase D body (R7.2 surface stability concern).
- (b) Models screen surfaces a "compare these checkpoints in Lab"
  button (Phase E follow-up).
- **(c) Memory entries can link back to Trail rows** — additive
  only (Memory is new in Phase F; the chevron lives on a new
  card). **Analyst-recommended default**.

**R6.3** — **Models → ?** — there is no Phase E follow-up at v0.1.0
(Q6b deferred to v0.2.0). Cell-click in the Models screen at v0.1.0
is a no-op (the per-checkpoint detail view per dev-note §J8 lines
616-622 is reserved for v0.2.0 per R2.2 framing).

### R7 — Non-regression contract

**R7.1** — **22 body-SHA-256 anchors stay byte-identical**. Phase F
touches no strategy / audit / exec / report-renderer path. Memory
+ Models are pure read surfaces over data that other features (the
reflection writer, the v2.5 TCN training loop) generate. The
Assistant slot at Q4=(a) renders only static placeholder copy. H2
carry-forward from Phase D+ predecessor applies verbatim.

**R7.2** — **Phase A/B/C/D/D+/E-shipped surfaces byte-identical** —
specifically:
- Lab screen body (Phase A + B) — unchanged.
- Live screen (Phase C) — unchanged.
- Strategy registry (Phase C) — unchanged.
- Settings (Phase C) — unchanged.
- Trail screen (Phase D + D+) — unchanged at v0.1.0 (R6.2 chooses
  default (c) to avoid Phase D body churn; if operator overrides
  to (a) or (b), Phase D body changes and R7.2 requires re-baselining
  the trail snapshot).
- Compare matrix (Phase E) — unchanged.
- Sidebar (Phase C) — Memory + Models entries already wired; only
  the body routes swap.
- Right-rail slot — at Q4=(a) the closed-state shell composition is
  byte-identical (`RIGHT_RAIL_WIDTH_PX = 0.0` is preserved for
  `AssistantState::is_open == false`); only the open-state widens
  the right column. Default state on cockpit boot is `is_open ==
  false` so all existing snapshots are byte-identical to ship.

**R7.3** — **`cockpit-smoke` PASS 0 panics** — Memory + Models +
Assistant slot active states all render under the layout-invariants
proptest (R8.4 below).

**R7.4** — **`cockpit-performance v1.0.0` idle-CPU floor ≤ 13.6 %**
preserved (Phase D+ baseline: 13.1 % floor + 0.5 % headroom).
Phase F renders are on-demand (Memory + Models render only when
their screen is active; Assistant slot renders only when
`is_open == true`); no new periodic widget; no new subscription.
H3 below quantifies. Cold-boot reads (R5.1, R5.2) happen once per
screen-open per session; H1 + H2 quantify the budget.

**R7.5** — **`spec-lint` Phase F contribution = 0** — baseline from
Phase E is ≤ 91 / 2 categories; Phase F adds no new dead-link rows
and no new trace-broken-path rows.

**R7.6** — **No new external crate deps; no new Lumen tokens
(except R3.4's `RIGHT_RAIL_OPEN_WIDTH_PX`); no iced bump.** Vendored
`iced_tiny_skia` fork stays untouched per CLAUDE.md operator-lock
(2026-05-20).

**R7.7** — **No backtest binary changes; no anchored renderer touch;
no audit-ledger writer touch; no reflection-memory writer touch.**
Phase F is read-only over data the reflection writer + the forecast
training loop already produce.

### R8 — Public API surface added

**R8.1** — New `Message` variants (gated on Q4):
- `Message::MemoryToggleMode(MemoryViewMode)` — toolbar toggle.
- `Message::MemorySetFilter(Option<MemoryFilter>)` — filter chip.
- `Message::MemoryOpenTrail(audit_id)` — alias for `OpenTrailFor`
  with the Memory→Trail cross-link semantics (or just reuse
  `OpenTrailFor` directly — architect M-T1 to decide).
- `Message::ModelsSetFamilyFilter(Vec<ModelFamily>)` — toolbar.
- `Message::ModelsSetStatusFilter(Vec<ModelStatus>)` — toolbar.
- `Message::ToggleAssistantSlot` — collapse/expand (gated on Q4
  ≠ (c)).

**R8.2** — New enums (pure data): `MemoryViewMode`, `MemoryFilter`,
`ModelFamily`, `ModelStatus`, `AssistantMode` (if Q4 ≠ (c)).

**R8.3** — New structs (pure data, default-constructible):
`MemoryScreenState`, `ModelsScreenState`, `AssistantState` (if Q4
≠ (c)), `LessonCardCard` (memory-row view-model — distinct from
`LessonCard` to avoid leaking the reflection-crate type into the UI),
`CheckpointMeta` (models-row view-model).

**R8.4** — New modules (mirrors the Phase E layout):
- `crates/ui/src/memory/` with `mod.rs`, `state.rs`, `store_read.rs`.
- `crates/ui/src/models/` with `mod.rs`, `state.rs`, `registry_read.rs`.
- `crates/ui/src/assistant/` with `mod.rs`, `state.rs` (gated on Q4
  ≠ (c)).
- `crates/ui/src/screens/memory.rs` (replaces placeholder route).
- `crates/ui/src/screens/models.rs` (replaces placeholder route).

**R8.5** — Net-new file count: 8-10 (3 screens, 0-3 widget files
depending on cluster-mode shape, 4-7 module files). Architect M-T1
to lock exact count. Q4=(c) reduces to 6-7 (no assistant module).

## Q-questions (operator-decide)

### Q1 — Memory screen body shape

(a) **Reverse-chronological journal entries list** — one card per
    `LessonCard` ordered by `closed_at DESC`; matches dev-note §J7
    line 569 "list view, cards mode default".
(b) Entity-grouped — collapsible groups by strategy or symbol;
    operator scans the first card in each group.
(c) Search-first with filter chips — text-search + filter UI as the
    primary affordance; cards rendered as a search result list.

**Analyst-recommended: (a)** — matches dev-note §J7 verbatim (line
569 "list view"); simplest cold-boot UX; filter chips (R1.2 toolbar)
can be additive later. Surfaces (b) + (c) for operator override.

### Q2 — Models screen body shape (when v2.5 has landed checkpoints — current state)

(a) **Flat checkpoint list** — one row per checkpoint discovered
    on disk; columns per dev-note §J8 lines 605-614; sortable.
(b) Timeline view — training runs grouped by date; visual cluster
    by month/quarter.
(c) Registry-table with hash+date+anchor-status columns — emphasis
    on the SHA + anchor cross-link (which anchors lock this
    checkpoint's outputs?).

**Analyst-recommended: (a)** — matches dev-note §J8 verbatim (line
604 "list view, one row per trained model checkpoint"); the column
set (Q2-a) already covers (c)'s most-load-bearing column (the SHA);
(b) ships only when the v2.5a / v2.5b families add density to the
list (≥ 5 checkpoints).

### Q3 — Models screen behavior when no checkpoints are present in the running tree

(a) **Honest "no models loaded" placeholder** with a "How to add"
    link to `spec/v25-tcn-overlay/feature.md` — matches Phase A/C
    placeholder precedent; sidebar entry stays visible.
(b) Hide the screen entirely — sidebar entry greyed out — breaks
    Phase C IA (sidebar grouping invariant per `theme.rs:737-739`
    `sidebar_groups_phase_c__flatten_matches_phase_a`).
(c) Populate from a hardcoded demo fixture — dishonest about the
    real state.

**Analyst-recommended: (a)** — honest + discoverable; preserves
sidebar IA invariant. At 2026-05-20 the BS-1 + BS-2 checkpoints are
on disk (confirmed in feature.md §"Why" item 2) so the live screen
populates with 2 rows on the operator's machine; this question is
about the fresh-checkout / CI / cold-start UX.

### Q4 — Assistant slot v0 behavior (Lumen Phase 6 wake)

**Grounding**: v2 LLM strategy (`v2-llm-strategy v2.0.0`)
**shipped 2026-05-13** (backlog.md:1207-1213) — the gate from
"Phase 6 reserved" → "Phase 6 eligible" is **met**. The decision
shifts from "wait for v2 LLM" to "how much of the slot do we wake at
v0.1.0".

(a) **Stub-only** — `RIGHT_RAIL_WIDTH_PX` flips to a real width when
    `AssistantState::is_open == true`; body renders "Assistant
    offline" placeholder; v2 LLM wiring lands in v0.2.0. **Analyst-
    recommended**: lights the slot honestly without scope-creep
    into LLM plumbing in the same pass.
(b) Wire to v2 LLM via minimal text-stream input/output — uses
    `crates/llm::AnthropicProvider` (already in the workspace per
    `crates/llm/src/providers/anthropic.rs`); ~1 week of additional
    scope (LLM cost-budget wiring + streaming surface inside iced);
    moves Phase F cost estimate from 3-4 weeks to 4-5 weeks.
(c) Skip the Assistant slot entirely — Phase F ships only Memory +
    Models; right-rail stays at `Length::Fixed(0.0)`; follow-up
    brief `ui-rethink-phase-f-assistant-wire` lands separately
    (v0.2.0).

**Analyst-recommended: (a)**. Rationale: (a) closes the dev-note's
"Lumen Phase 6" deferred item with **zero LLM-plumbing scope** in
Phase F; the right-rail wakes structurally, the body is honest
placeholder copy. (b) is the lighter-weight default if the
operator's pattern is "ship the wire too" — but the v2 LLM ship is
only 1 week old at this pass, and a chat-shaped consumer is a
materially different surface from the v2 LLM strategy/pipeline
shape. (c) leaves the rethink with one un-shipped deliverable per
dev-note §6 Phase F bullet 3 — surfaced as the "minimum scope"
option if the operator wants to ship Memory + Models cleanly first
and tackle Assistant as a separate sweep.

### Q5 — Memory entry detail view

(a) Inline expand-row — clicking a card expands it in place; no
    new screen/drawer; constrained to the card's column width.
(b) **Side drawer** — mirrors the Phase D Trail drawer
    (`widgets/trail_drawer.rs`); reuses `RIGHT_RAIL_WIDTH_PX`
    family of layout constants; consistent with cockpit precedent.
(c) Dedicated route — `Screen::MemoryEntry(id)`; adds a route to
    the screen enum; deeper navigation tree but heavier.

**Analyst-recommended: (b)** — drawer is the precedent set by
Phase D; reuses the same right-rail family of layout primitives;
no new `Screen` variant. Note: if Q4=(a) the right-rail is already
"the Assistant slot" — a Memory drawer would need to share that
space or use a left-side drawer instead. Architect M-T1 to confirm
the layout coexistence with Q4=(a). If conflict surfaces, fall
back to (c) (`Screen::MemoryEntry`) for v0.1.0.

### Q6 — Cross-link surfaces

(a) Trail (Phase D) cells link into Memory entries when relevant —
    touches Phase D body (R7.2 surface stability concern).
(b) Models screen surfaces a "compare these checkpoints in Lab"
    button (Phase E follow-up) — touches Lab/Compare bodies (R7.2
    again).
(c) **Memory entries can link back to Trail rows** — additive only
    (Memory is new in Phase F; the chevron lives on a new card).
    No Phase D body change.

**Analyst-recommended: (c)** — Trail is already the entry point for
J4 (per dev-note §J4); Memory adds a back-link so the operator can
trace "this lesson came from this trade" without re-grepping. (a) +
(b) are operator-overrides that lift scope.

### Q7 — Models screen "serving" pill semantics (sub-decision under R2.2)

The status pill per dev-note §J8 line 612 has three values:
`serving / staged / archived`. The dev-note does not define how
each is determined. Options:

(a) `serving` = mentioned by name in any
    `config/strategies/*.toml` running strategy at boot; `archived`
    = explicitly moved to `crates/forecast/checkpoints/archived/`
    subfolder; `staged` = everything else.
(b) `serving` = the checkpoint loaded into a live
    `ForecastProvider` at runtime (would need new state plumbing
    from the strategy runtime); v0.1.0 always "unknown".
(c) **All checkpoints on disk render as `staged`** at v0.1.0 with
    a tooltip "Lifecycle classification ships in v0.2.0".

**Analyst-recommended: (c)** for v0.1.0 — keeps the screen honest
and the scope tight; (a) requires the architect to wire the config
parse into the UI (~ 1 day) and (b) requires runtime-state plumbing
which is anchor-risky. v0.2.0 follow-up lifts to (a).

### Q8 — Memory `list_recent_lesson_cards` read API placement (sub-decision under R5.1)

(a) Extend the `ReflectionStore` trait with `async fn list_recent(&self,
    limit: usize) -> Result<Vec<LessonCard>, ReflectionStoreError>` —
    additive on the trait; touches every impl (currently just
    `SqliteReflectionStore`).
(b) **Write the SQL query directly inside `crates/ui/src/memory/store_read.rs`**
    (bypass the trait); one-call site; v0.2.0 can lift to (a) if a
    second consumer appears.
(c) Add a `list_recent` method on `SqliteReflectionStore` (the impl)
    only — bypasses the trait but keeps the read inside the
    reflection crate.

**Analyst-recommended: (b)** — keeps the trait surface minimal; the
UI is the one read consumer at v0.1.0; if v0.2.0 adds an audit-mirror
consumer or a reflection-distillation consumer, lift to (a). Note:
(c) is the "middle ground" that keeps the SQL inside the reflection
crate but doesn't promote it to the trait; surface for operator
review.

## K-risk register

### K1 — Reflection-memory read API absence
**Risk:** The `ReflectionStore` trait at
`crates/reflection/src/store/mod.rs:27-35` exposes only `upsert /
top_k / count` — there is no `list_all` or `list_recent_n`. Memory
screen at R1.2 needs to render N recent cards (default N = 50 per
Q1=a reverse-chronological list). Architect must choose between Q8
options (a) / (b) / (c).
**Severity:** LOW (resolved by Q8 operator-decide).
**Mitigation:** Q8 surfaces the three options with analyst-recommended
(b) — direct SQL query in `memory/store_read.rs` keeps the trait
surface minimal. Architect M-T1 to lock the choice and document in
`decomp.md`. Fail-soft: if the query fails, the Memory screen
renders the empty-state placeholder (R1.4) with a `tracing::error!`
breadcrumb.

### K2 — Checkpoint metadata schema drift
**Risk:** The Models screen reads
`crates/forecast/checkpoints/anchors/*.metadata.json` files. The
schema is owned by the v2.5 TCN training loop — Phase F is a passive
consumer. If a v2.5a (PatchTST) or v2.5b (Transformer) checkpoint
writes a different schema, the Models screen may crash or render
malformed rows.
**Severity:** LOW.
**Mitigation:** `discover_checkpoints` (R5.2) returns `Option<CheckpointMeta>`
per file; parse failures emit a `tracing::warn!` with the offending
path and the file is skipped. Architect M-T1 to inventory the
current `metadata.json` schema and lock a parser shape that's
robust to additional/missing optional fields (serde `#[serde(default)]`
on every non-load-bearing field).

### K3 — Forecast-quality sparkline data absence
**Risk:** R2.2 lists a per-checkpoint forecast-quality sparkline
(dev-note §J8 line 613-614, "last-N-bars actual vs predicted
residuals"). The residual series may not be on disk at v0.1.0 ship
time (the dev-note investigation at line 1175 names
`crates/replay-cache/` namespace `"forecast"` as the source — but
this namespace may be empty for the v2.5 TCN BS-1/BS-2 checkpoints
at this pass).
**Severity:** LOW.
**Mitigation:** R2.2 explicitly defers the sparkline to v0.2.0 if
data is absent. Phase F v0.1.0 ships the row layout with a `—`
placeholder where the sparkline would render; tooltip "Forecast
quality ships when residual cache populates". Architect M-T1 to
grep the replay-cache for "forecast" namespace entries; if present,
sparkline ships at v0.1.0; if absent, deferred and noted in the
presenter deck.

### K4 — Assistant slot layout conflict with Q5=(b) drawer
**Risk:** Q5=(b) puts the Memory entry detail in a right-side drawer.
Q4=(a) wakes the right-rail Assistant slot. Both compete for the
right side of the cockpit. If the operator opens the Assistant slot
AND clicks a Memory chevron, the drawer + slot may overlap, occlude,
or break layout.
**Severity:** MEDIUM (UX trap surfaced honestly).
**Mitigation:** Architect M-T1 to decide the coexistence rule:
either (i) Memory drawer pushes the Assistant slot temporarily (slot
auto-collapses when drawer opens), (ii) Memory drawer renders on
the left side (mirrors the sidebar nav side; novel pattern), or
(iii) Q5 falls back to (c) (`Screen::MemoryEntry` route) which has
no drawer at all. Analyst-recommended fallback: (iii) if conflict
material; document in `decomp.md`. Q-Future visual treatment: a
Lumen modal scrim when both surfaces would conflict.

### K5 — Cold-boot read cost (Memory + Models)
**Risk:** Memory cold-boot reads N lesson cards from the
sqlite store; Models cold-boot scans the checkpoint folder + parses
N json files. If N is large (>1000 cards, >100 checkpoints), the
cold-boot stall could be >100 ms on first screen-open.
**Severity:** LOW.
**Mitigation:** At 2026-05-20 N is small (lesson cards from the
last few weeks of paper-trading; 2 checkpoints on disk). H1 + H2
below quantify. If H1 or H2 falsify the budget, the cold-boot scan
moves to a background `tokio::spawn` at cockpit boot (mirrors Phase
E K5 mitigation). No anchor risk either way.

### K6 — `RIGHT_RAIL_WIDTH_PX` constant semantic change
**Risk:** R3.4 changes `RIGHT_RAIL_WIDTH_PX` from a constant to a
function-of-state. Any existing call site that assumes the constant
is `0.0` (Phase D trail-drawer at
`crates/ui/src/widgets/trail_drawer.rs:70,175,179`) may now see a
non-zero value when the Assistant slot is open, potentially
double-counting the right column in layout math.
**Severity:** MEDIUM (cross-feature coupling — Phase D trail-drawer
already references `RIGHT_RAIL_WIDTH_PX`).
**Mitigation:** Architect M-T1 to trace every `RIGHT_RAIL_WIDTH_PX`
reference and decide the migration:
- Option A: keep `RIGHT_RAIL_WIDTH_PX = 0.0` as the closed-state
  constant; add a NEW constant `RIGHT_RAIL_OPEN_WIDTH_PX`; the shell
  picks one based on `assistant_state.is_open`. Trail drawer
  continues using `RIGHT_RAIL_WIDTH_PX = 0.0` unchanged (the closed
  semantic).
- Option B: replace the constant with a function `right_rail_width(state)`;
  every call site (including trail-drawer) updates to pass the
  state. More refactor; less risk of stale closed-state values.

Analyst-recommended Option A — minimal churn; trail-drawer (Phase D)
body stays byte-identical (R7.2). Document in `decomp.md`.

### K7 — Assistant slot wake without v2 LLM body wired (Q4=(a))
**Risk:** Operator opens the Assistant slot at Q4=(a), sees
"Assistant offline" placeholder, expects to type a prompt or see
recent agent activity, and is disappointed. UX trap: the wake
without wire creates expectation without payoff.
**Severity:** LOW (subjective UX).
**Mitigation:** R3.2 (a) body copy explicit: "Assistant offline.
v2 LLM wiring lands in v0.2.0." plus a link to the v2 LLM presenter
deck (`spec/v2-llm-strategy/presentations/v2-llm-strategy-2026-05-13.md`)
so the operator can see what shipped 2026-05-13. The toggle button
includes a "(coming soon)" suffix at v0.1.0. v0.2.0 lifts the
placeholder once R3.2 (b) wiring ships.

### K8 — Phase F final-sweep "anything missing?" gap risk
**Risk:** Per dev-note §6 line 1110, Phase F is the final "anything
missing?" sweep. The operator's presenter review may surface a job-
story or surface that wasn't enumerated in the dev-note's J1-J8 +
§3 IA — e.g. a J9 "monitor agent costs" or J10 "tune risk envelope"
that should ship as part of the rethink rather than as a v0.2.0
brief.
**Severity:** LOW (process, not code).
**Mitigation:** Presenter deck includes an explicit "Anything missing?"
slide listing the 8 dev-note job-stories (J1-J8) + the 6 phases
shipped (A-F) + a "Gaps?" prompt for operator review. If a gap
surfaces, it becomes a follow-up brief under the rethink umbrella
(naming convention: `ui-rethink-phase-f-<gap-slug>` or
`ui-rethink-phase-g-<new-job-story>`).

## H-hypothesis register

### H1 — Memory cold-boot read budget < 50 ms p99
**Claim:** Reading N=50 most-recent `LessonCard` rows from the
sqlite-backed `SqliteReflectionStore` completes in < 50 ms p99 on
the operator's typical workstation at 2026-05-20 scale.
**Falsification:** Architect M-T1 enumerates the current
`lesson_cards` row count (`sqlite3 .../reflection.db 'SELECT COUNT(*)
FROM lesson_cards'`) and runs a static argument based on the row
count + the in-process `top_k` deterministic-linear-scan precedent
at `store/sqlite.rs:5-6` ("R7.2 sized for the v1 ≤500-card budget").
If row count is >500 or query p99 > 50 ms, K5 mitigation lifts
(background `tokio::spawn` at boot).
**Why this number:** the v1 reflection-memory crate is explicitly
sized for ≤500 cards (per the `sqlite.rs:5-6` annotation); a 50-row
read over 500 total rows on sqlite WAL mode is microsecond-scale in
benchmark precedent; 50 ms is the "first-open feels instant"
threshold.

### H2 — Models cold-boot scan budget < 50 ms p99
**Claim:** Globbing
`crates/forecast/checkpoints/anchors/*.metadata.json` + parsing each
JSON file completes in < 50 ms p99 at 2026-05-20 scale (2 files:
`tcn-bs1-*.metadata.json` + `tcn-bs2-*.metadata.json`).
**Falsification:** Architect M-T1 micro-bench runs the scan path
against the live `crates/forecast/checkpoints/anchors/` directory;
if p99 > 50 ms, K5 mitigation lifts. Static argument: 2 small JSON
files (likely < 4 KB each) + serde_json deserialize is microsecond-
scale; 50 ms has ~1000× headroom.
**Acceptable fallback:** background scan at cockpit boot; no UI
gating either way.

### H3 — Idle-CPU floor preserved
**Claim:** Memory + Models + Assistant slot (closed-state) renders
are **on-demand** (no new periodic widget, no new subscription);
when these screens are not active AND the Assistant slot is closed,
they consume zero CPU. Idle CPU floor stays ≤ 13.6 % (Phase D+
baseline).
**Falsification:** Tester runs cockpit-performance v1.0.0 with
Phase F applied and Memory as the active screen for 60 s; if idle
CPU > 14.6 % (13.6 % + 1 % budget) H3 is falsified. Same test
repeated with Models as the active screen and with Assistant slot
open. Static argument: no new `tokio::time::interval`, no new
subscription producer; all screens re-render only on `Message`
arrival — same model as Phase C / D / E which all hit the budget.

### H4 — Reflection-memory `list_recent` query correctness
**Claim:** The Q8=(b) direct-SQL query in
`crates/ui/src/memory/store_read.rs` returns the N most-recent
`LessonCard` rows ordered by `closed_at DESC` correctly, matching
the schema declared at `crates/reflection/migrations/001_lesson_cards.sql`.
**Falsification:** Unit test in
`crates/ui/src/memory/store_read.rs` `#[cfg(test)] mod tests`
populates an in-memory sqlite with 5 fixture rows at known
timestamps; calls `list_recent_lesson_cards(store, 3)`; asserts the
returned 3 rows are the 3 most-recent by `closed_at DESC`. Identical
shape to Phase E's cache unit tests at
`crates/ui/src/compare/cache.rs:#[cfg(test)] mod tests`.

### H5 — Checkpoint metadata schema robustness
**Claim:** The Q4=(a) sparkline (or its placeholder) does not crash
the Models screen when `metadata.json` is missing a non-load-bearing
field (e.g. `sigma_train` absent). The parser tolerates schema
drift by emitting `None` for missing optional fields.
**Falsification:** Unit test in
`crates/ui/src/models/registry_read.rs` `#[cfg(test)] mod tests`
parses 3 fixture JSONs: (i) full schema, (ii) missing
`sigma_train`, (iii) malformed (returns `None`). Asserts (i) +
(ii) parse successfully with `sigma_train = None` in case (ii);
asserts (iii) returns `None`.

### H6 — Right-rail layout invariant under Q4=(a) wake
**Claim:** Toggling `AssistantState::is_open` between `false` and
`true` does not cause layout panics, zero-dim widgets, or visible
flicker. The cockpit-smoke layout-invariants proptest (256 viewport
samples) passes for both states.
**Falsification:** New layout-invariants proptest case
`assistant_slot_open_no_zero_dim` in
`crates/ui/tests/layout_invariants.rs`. 256 cases × {open, closed}
= 512 total; assert no panic + every layout primitive resolves to
positive dimensions when the state allows them to be visible.

## Non-regression contract

1. **22 body-SHA-256 anchors stay byte-identical** (R7.1).
2. **Phase A/B/C/D/D+/E-shipped surfaces byte-identical** (R7.2) —
   Lab body, Live, Strategy registry, Settings, Trail, Compare,
   sidebar 3-zone grouping all unchanged. Right-rail slot closed-
   state preserves the existing `Length::Fixed(0.0)` shell composition
   (K6 mitigation Option A).
3. **`cockpit-smoke` PASS 0 panics** under Memory + Models +
   Assistant-open active states (R7.3, H6).
4. **`cockpit-performance v1.0.0` idle-CPU floor ≤ 13.6 %** preserved
   (R7.4, H3).
5. **`spec-lint` Phase F contribution = 0** (R7.5).
6. **No new external crate deps; no new Lumen tokens (except
   `RIGHT_RAIL_OPEN_WIDTH_PX` if Q4 ≠ (c)); no iced bump** (R7.6).
   `iced_tiny_skia` vendored fork stays untouched per CLAUDE.md
   operator-lock 2026-05-20.
7. **No backtest binary changes; no anchored renderer touch; no
   audit-ledger writer touch; no reflection-memory writer touch**
   (R7.7).
8. **Backtest determinism preserved** — Phase F does not invoke the
   engine, does not write reports, does not call into the
   reflection writer pipeline.

## Acceptance criteria

### M0 — Analyst synthesis (this pass)
- [x] R1..R8 anchored to dev-note §6 Phase F (lines 1098-1112) +
      §J7 (lines 561-595) + §J8 (lines 596-637).
- [x] Q1-Q8 surfaced with analyst-recommended defaults.
- [x] K1-K8 risk register; K4 (assistant-slot + memory-drawer
      coexistence) + K6 (right-rail constant semantic change)
      surfaced as the load-bearing UX/coupling traps.
- [x] H1-H6 falsifiable hypotheses (per-screen latency budgets +
      data-shape robustness + layout invariance).
- [x] Non-regression contract enumerated (8 items).
- [x] Predecessor surfaces audited:
      - Phase C sidebar IA reserves `Screen::Memory` +
        `Screen::Models` in `SIDEBAR_GROUPS_PHASE_C` Library zone
        (`theme.rs:741-750`).
      - `screens::memory` + `screens::models` route to
        `placeholder::view` at `shell.rs:98-99`.
      - `strings::MEMORY_PLACEHOLDER` + `strings::MODELS_PLACEHOLDER`
        + `strings::SIDEBAR_NAV_MEMORY` + `strings::SIDEBAR_NAV_MODELS`
        already exist (`strings.rs:258-275`).
      - Right-rail Phase 6 Assistant slot reservation at
        `shell.rs:47-49` + `theme.rs:640-643` (Phase 2 Q7 ratification).
- [x] v2 LLM ship status confirmed (Q4 grounding):
      `v2-llm-strategy v2.0.0` shipped 2026-05-13 — backlog.md:1207-1213.
- [x] Checkpoint presence confirmed (Q3 grounding):
      `tcn-bs1` + `tcn-bs2` `.safetensors + .metadata.json` on disk
      at `crates/forecast/checkpoints/anchors/`; locked under
      `forecast-distribution-bs1-realdata` /
      `…-bs2-realdata` anchors (`spec/anchors.toml:156-161`).
- [x] Reflection-memory crate audited: trait at
      `crates/reflection/src/store/mod.rs:27-35` exposes only
      `upsert / top_k / count` — Q8 needed to choose the read path
      shape.
- [x] Trace row `REQ-UI-RETHINK-PHASE-F-001` to be opened in
      `draft` state by this pass.

### M-OD — Operator-decide (Q1-Q8)
- [ ] Q1 — Memory screen body shape (analyst-recommended: a —
      reverse-chronological list).
- [ ] Q2 — Models screen body shape (analyst-recommended: a — flat
      list).
- [ ] Q3 — Models screen when no checkpoints (analyst-recommended:
      a — honest placeholder).
- [ ] Q4 — Assistant slot v0 behavior (analyst-recommended: a —
      stub-only wake).
- [ ] Q5 — Memory entry detail view (analyst-recommended: b — side
      drawer, fallback to c if K4 conflict).
- [ ] Q6 — Cross-link surfaces (analyst-recommended: c — Memory →
      Trail back-link only).
- [ ] Q7 — Models "serving" pill semantics (analyst-recommended: c
      — all `staged` at v0.1.0, lifecycle in v0.2.0).
- [ ] Q8 — Memory `list_recent` read API placement (analyst-
      recommended: b — direct SQL in UI module, bypass trait).

### M-T1 — Architect decomposition
- [ ] Architect resolves K1 + Q8 (read API placement) and locks the
      `list_recent_lesson_cards` shape.
- [ ] Architect resolves K2 + Q7 (status pill semantics) and locks
      the `discover_checkpoints` parser shape (serde defaults).
- [ ] Architect resolves K3 (sparkline data presence) by grepping
      `crates/replay-cache/` for the `"forecast"` namespace.
- [ ] Architect resolves K4 + Q5 (drawer vs Assistant slot layout
      coexistence) and documents in `decomp.md`.
- [ ] Architect resolves K6 (right-rail constant semantic) — Option
      A (new `RIGHT_RAIL_OPEN_WIDTH_PX`) recommended by analyst.
- [ ] Architect runs H1 enumeration: count `lesson_cards` rows in
      the operator's live reflection.db; record in `decomp.md`.
- [ ] Architect runs H2 micro-bench: checkpoint metadata scan p99;
      record in `decomp.md`.
- [ ] Architect decomposes R1-R8 into ordered T-D-N tasks per wave.
      Suggested wave map:
      - Wave A = state modules (memory/state, models/state, assistant/state if Q4≠c) + Message variants
      - Wave B = read modules (memory/store_read, models/registry_read)
      - Wave C = `screens::memory` + shell wiring
      - Wave D = `screens::models` + shell wiring
      - Wave E = `assistant` slot wake + shell wiring (skip if Q4=c)
      - Wave F = snapshot baselines (1-4 per screen) + layout-invariants proptest cases + cockpit-smoke pre-run
      - Wave G = anchor gate + spec-lint sweep + tester handoff envelope
- [ ] Architect confirms net-new file count (R8.5).
- [ ] Architect closes Q5 sub-decision on drawer-vs-route under
      Q4=(a) layout coexistence.

### M-FINAL — Tester sweep
- [ ] `cargo fmt --check` + `cargo clippy --workspace -- -D warnings`
      exit 0.
- [ ] `cargo test --workspace --lib` 100 % PASS.
- [ ] New snapshot baselines (per-screen, target count: 1-2 cold-
      boot + 1-2 populated per screen):
      - `memory__cold_boot_empty`
      - `memory__steady_state_5_cards`
      - `memory__drawer_open_on_card_click` (if Q5=b)
      - `models__cold_boot_no_checkpoints` (Q3=a placeholder)
      - `models__steady_state_2_checkpoints` (live BS-1 + BS-2 path)
      - `assistant_slot__closed_default` (byte-identical to existing shell baselines per K6 Option A)
      - `assistant_slot__open_stub` (if Q4=a)
- [ ] `scripts/verify_anchors.sh` → 22/22 PASS — non-negotiable
      (R7.1).
- [ ] `cockpit-smoke` → 0 panic lines on Memory + Models +
      Assistant-open active screens (R7.3).
- [ ] Cockpit-performance v1.0.0 idle-CPU floor ≤ 13.6 % preserved
      under each new active screen (R7.4, H3).
- [ ] H1 + H2 cold-boot read benchmarks recorded in test report.
- [ ] H4 unit test (`list_recent_lesson_cards_returns_n_recent`)
      PASS.
- [ ] H5 unit test (`discover_checkpoints_tolerates_schema_drift`)
      PASS.
- [ ] H6 layout-invariants proptest (`assistant_slot_open_no_zero_dim`)
      PASS.
- [ ] Author
      `spec/ui-rethink-phase-f-memory-models-assistant/reports/test-final-<YYYY-MM-DD>.md`.

### M-PRESENTER — Final sweep ("anything missing?")
- [ ] Presenter deck enumerates the 8 dev-note job-stories (J1-J8)
      and maps each to the phase / screen that shipped it (per K8
      mitigation).
- [ ] Presenter deck enumerates the 6 phases (A-F) and confirms
      each is byte-identical at ship (anchor gate carry-forward).
- [ ] Presenter prompts operator on "anything missing?" — any gap
      surfaced becomes a `ui-rethink-phase-f-<gap-slug>` or
      `ui-rethink-phase-g-<new-job>` follow-up brief.
- [ ] Operator-approval via "Autoapprove all" pattern OR explicit
      sign-off on the missing-coverage prompt.

## Cost estimate

Per dev-note §6 Phase F (line 1112): **~3-4 weeks**. **No cliffs**
(line 1134); independently shippable; independently reversible
(revert each screen body individually + reset
`Cockpit::memory_screen_state` / `Cockpit::models_screen_state` /
`Cockpit::assistant_state` fields).

Cost contingency: if Q4=(b) (full v2 LLM wire) is chosen, Phase F
cost rises to **~4-5 weeks** (added scope: LLM streaming surface
inside iced + cost-budget plumbing).

Anchor risk: **zero** (purely additive UI surface; no backtest
binary changes, no anchored renderer touch, no audit writer touch,
no reflection writer touch). 22-anchor regression gate carry-forward
H2 from Phase D+ → Phase E predecessor.

## Trace

Trace row `REQ-UI-RETHINK-PHASE-F-001` to be opened in `draft`
state by this analyst pass. `arch`, `crates`, `tests`, `anchors`
columns to be filled by architect / developer / tester respectively.

## Implementation

Developer Wave A-F complete 2026-05-21. Summary of what shipped:

### Wave A — State modules, Message variants, theme constant

- `crates/ui/src/memory/{mod,state}.rs` — `MemoryScreenState`, `MemoryViewMode`, `LessonCardCard` view-model.
- `crates/ui/src/models/{mod,state}.rs` — `ModelsScreenState`, `ModelFamily`, `ModelStatus`, `CheckpointMeta` view-model.
- `crates/ui/src/assistant/{mod,state}.rs` — `AssistantState`, `AssistantMode`.
- `crates/ui/src/theme.rs` — `RIGHT_RAIL_OPEN_WIDTH_PX = 320.0` added; `RIGHT_RAIL_WIDTH_PX = 0.0` preserved unchanged (K6 Option A).
- `crates/ui/src/state.rs` — 3 new `Cockpit` fields + 9 `Message` variants + 9 update arms.
- `crates/ui/src/strings.rs` — 12+ Phase F string constants; `MEMORY_PLACEHOLDER` + `MODELS_PLACEHOLDER` deprecated.
- `crates/ui/src/lib.rs` — 3 new module declarations.

### Wave B — Read modules

- `crates/reflection/src/query.rs` — `list_recent_lesson_cards` + `open_and_list_recent` convenience function. The convenience function returns `Ok(vec![])` immediately when `db_path.exists()` is false (cold-empty boot). Keeps sqlx encapsulated inside the reflection crate; the ui crate has no direct sqlx dep. H4 unit test (`list_recent_lesson_cards_returns_n_recent`) lives here: 5 fixture rows inserted, limit=3, asserts 3 most-recent by `closed_at DESC`.
- `crates/ui/src/models/registry_read.rs` — `discover_checkpoints` + `parse_metadata` + 3 serde structs (`CheckpointMetadata`, `CheckpointArchitecture`, `CheckpointDataSpan`) with `#[serde(default)]` on every non-load-bearing field (K2 mitigation). H5 unit tests: 5 cases (full, missing-dropout, missing-sigma, malformed, unknown-family). All 5 pass.
- Q8=(b) refined placement: SQL lives in `crates/reflection/src/query.rs` (sibling of `store/`), not in `crates/ui/` — honors Q8=(b) "no trait change" while respecting that the UI crate has no tokio runtime.

### Wave C — Memory screen + drawer + shell wiring

- `crates/ui/src/screens/memory.rs` — Toolbar (Cards/Cluster toggle; Cluster disabled), cards list, optional side-drawer (Q5=b).
- `crates/ui/src/memory/drawer.rs` — Side-drawer body mirroring Phase D `widgets/trail_drawer.rs` composition. Width = `RIGHT_RAIL_OPEN_WIDTH_PX = 320.0`.
- `crates/ui/src/shell.rs` — `Screen::Memory` swapped from `placeholder::view` to `memory::view`.

### Wave D — Models screen + shell wiring

- `crates/ui/src/screens/models.rs` — Toolbar (TCN active chip; PatchTST/Transformer disabled chips; Staged status chip), checkpoint list (empty-state when `filtered.is_empty()`). Each row: family | rev (8 chars) | data span | status ("staged" per Q7=c) | sparkline ("—" per K3 deferral) | file size.
- `crates/ui/src/shell.rs` — `Screen::Models` swapped from `placeholder::view` to `models::view`.

### Wave E — Assistant slot wake + shell right-rail wiring

- `crates/ui/src/assistant/view.rs` — When `is_open == false`: returns 0-width Container (byte-identical to old Phase 2 reservation). When `is_open == true`: Lumen Phase 6 stub placeholder ("Assistant offline. v2 LLM wiring lands in v0.2.0."). K7 copy explicit.
- `crates/ui/src/shell.rs` — Right-rail `rail_width` is now a function of `assistant_state.is_open`: `Length::Fixed(RIGHT_RAIL_OPEN_WIDTH_PX)` when open; `Length::Fixed(RIGHT_RAIL_WIDTH_PX)` when closed. K6 Option A preserved: `RIGHT_RAIL_WIDTH_PX = 0.0` constant unchanged; all existing snapshots byte-identical at default closed state.
- `crates/ui/src/bin/cockpit_live.rs` — Cold-boot hydrate wiring: two `iced::Task::perform` boot tasks send `Message::MemoryHydrate` + `Message::ModelsHydrate` via the side-thread tokio runtime. Both gated by `#[cfg(feature = "live")]`.

### Wave F — Snapshot baselines + layout-invariants + round-trip tests

- `crates/ui/tests/visual_snapshots.rs` — 6 Phase F baselines added: `memory__cold_boot_empty`, `memory__steady_state_5_cards`, `memory__drawer_open_on_card_click`, `models__cold_boot_no_checkpoints`, `models__steady_state_2_checkpoints`, `assistant_slot__open_stub`. All 6 accepted on first run; 0 panics.
- `crates/ui/tests/fixtures/mod.rs` — 6 Phase F fixture builders.
- `crates/ui/tests/layout_invariants.rs` — 3 proptest cases: `memory_screen_no_zero_dim`, `models_screen_no_zero_dim`, `assistant_slot_open_no_zero_dim` (H6 falsification; 256 cases × 3 = 768 total). All pass.
- `crates/ui/src/state.rs` — 3 round-trip unit tests: `memory_hydrate_populates_cache_and_indexed`, `memory_open_drawer_sets_drawer_open`, `toggle_assistant_slot_flips_is_open`. All pass.
- `scripts/verify_anchors.sh` → `ANCHORS PASS (22 / 22)` (R7.1 gate).

### Key decisions made during implementation

- **Q8=(b) placement refined**: SQL placed in `crates/reflection/src/query.rs` (not in `crates/ui/src/memory/store_read.rs` as the spec name suggested) because the UI crate has no sqlx dep. The `open_and_list_recent` helper keeps sqlx encapsulated within the reflection crate and the UI calls it by name.
- **K6 Option A confirmed**: `RIGHT_RAIL_WIDTH_PX = 0.0` constant preserved unchanged. `shell_grid` invariant test passes unmodified.
- **K4 resolved (no conflict)**: Memory drawer lives in the centre column body; Assistant slot is the far-right shell track. Different shell columns — no coexistence conflict at v0.1.0. No auto-collapse needed.
- **chip_disabled lifetime**: Changed to take `String` (owned) and return `Element<'static, Message>` to resolve the local-variable lifetime issue with PatchTST/Transformer label strings.
- **T-D-N21 cockpit-smoke**: No `cockpit_smoke` integration test file exists in the suite. Acceptance satisfied via 6 panic-free visual_snapshots + 768 panic-free layout_invariants cases collectively.

### Deviations from spec (none material)

No material deviations. The one naming variance (`query.rs` in the reflection crate vs `store_read.rs` in the UI crate) follows the M-T1 decomp.md § 1.1 resolution (architect locked the placement as `crates/reflection/src/query.rs`). The spec's R5.1 wording ("new module `crates/ui/src/memory/store_read.rs`") was superseded by the architect's M-T1 refined resolution; tasks.md T-D-N8/N10 cite the final paths.

## Changelog

- 2026-05-20 (analyst): initial brief — R1-R8, Q1-Q8, K1-K8, H1-H6,
  non-regression contract; predecessor
  `ui-rethink-phase-e-compare v0.1.0`; scope anchored to dev-note
  §6 Phase F (lines 1098-1112) + §J7 (lines 561-595) + §J8 (lines
  596-637); v2 LLM ship status confirmed (v2.0.0 shipped 2026-05-13);
  checkpoint presence confirmed (BS-1 + BS-2 on disk at
  `crates/forecast/checkpoints/anchors/`); HANDOFF → operator-decide
  (Q1-Q8) → architect for M-T1 decomposition.
- 2026-05-21 (developer): Wave A-F complete (T-D-N1..N22 ticked); HANDOFF → tester for M-FINAL sweep.
