---
slug: ui-rethink-phase-f-memory-models-assistant
status: in-progress
owner: architect
updated: 2026-05-20
version: 0.1.0
predecessor: ui-rethink-phase-e-compare v0.1.0
---

# Decomposition — UI rethink Phase F (Memory + Models + Phase-6 Assistant slot, v0.1.0)

> Architect M-T1 pass. Resolves K1 (Memory read-path placement — refined
> Q8=(b) to live in `crates/reflection/src/query.rs` called by
> cockpit_live, results pushed to UI via a hydrate message; honors the
> "no trait change" spirit of Q8=(b) while respecting that the UI crate
> has no tokio runtime), K2 (checkpoint metadata schema — full serde
> shape with `#[serde(default)]` on every non-load-bearing field locked
> against the live `tcn-bs1` / `tcn-bs2` metadata.json files), K3
> (sparkline data — `replay-cache "forecast"` namespace empty at
> 2026-05-20, sparkline deferred to v0.2.0 per R2.2 framing — row layout
> ships with `—` placeholder + tooltip), K4 (drawer-vs-assistant
> coexistence — drawer auto-collapses assistant slot; both surfaces
> never co-exist; analyst-recommended fallback (c) not required), K6
> (RIGHT_RAIL constant — Option A confirmed: keep `RIGHT_RAIL_WIDTH_PX =
> 0.0` as the closed-state default, add `RIGHT_RAIL_OPEN_WIDTH_PX =
> 320.0` for open-state; Phase D trail_drawer references at
> `widgets/trail_drawer.rs:70,175,179` stay byte-identical — they continue
> using `RIGHT_RAIL_WIDTH_PX` which is unchanged at `0.0`). H1
> enumeration: live `data/audit/reflection.db` ABSENT (only
> `data/audit/ledger.db` exists at 2026-05-20); 0-row read budget is
> microsecond-scale by trivial argument; cold-empty placeholder (R1.4)
> is the dominant first-open UX. H2 micro-bench: 2 × ≤ 1 KB JSON files
> + serde parse << 1 ms by static argument; the analyst-claimed 50 ms
> budget has ~50000× headroom. Wave A-E ordered with T-D-N1..N18.
> Spike requirement = NONE (Memory + Models read paths use Phase D
> trail_mirror precedent verbatim; Assistant slot Q4=(a) renders static
> placeholder copy + a toggle — no LLM plumbing scope).
>
> Inputs reviewed:
> - `spec/ui-rethink-phase-f-memory-models-assistant/feature.md`
>   (R1-R8, K1-K8, H1-H6, Q1-Q8 — M-OD resolutions 2026-05-20:
>   Q1=a, Q2=a, Q3=a, Q4=a, Q5=b (drawer with K4 mitigation), Q6=c,
>   Q7=c, Q8=b (architect-refined: see § 1.1 below)).
> - `spec/ui-rethink-phase-f-memory-models-assistant/tasks.md` (T-A1..
>   T-A15 done; T-OD1..T-OD8 done; T-T1-*/T-D-N* this pass owns).
> - Predecessor `spec/ui-rethink-phase-e-compare/decomp.md`
>   (structural template; change-map shape + wave shape + rollback
>   shape carry forward 1:1; Phase F is structurally larger — 3
>   surfaces vs 1, 9 net-new source files vs 5).
> - `spec/dev-notes/ui-rethink-2026-05-17.md` §6 Phase F (lines
>   1098-1112), §J7 (lines 561-595 — reflection memory), §J8 (lines
>   596-637 — model versions), §6 Phase ordering (lines 1114-1140 —
>   "no cliffs … independently shippable and independently reversible").
> - Load-bearing source citations:
>   - `crates/ui/src/state.rs:55-67` — `Screen::Memory` / `Screen::Models`
>     enum variants (Phase A placeholder reservation; Phase F re-uses).
>   - `crates/ui/src/state.rs:692-708` — `TrailScreenState` (sibling
>     pattern; reconstructed_trail + pending_audit_id are the
>     "hydrated-or-pending" precedent for `MemoryScreenState.cache` and
>     `ModelsScreenState.checkpoints`).
>   - `crates/ui/src/state.rs:879,884,964-965,1015-1016,1115-1116` —
>     three-touchpoint pattern (struct field + Debug + 2× Default)
>     that `memory_screen_state` / `models_screen_state` /
>     `assistant_state` must replicate.
>   - `crates/ui/src/shell.rs:30,47-49,98-99` — RIGHT_RAIL_WIDTH_PX
>     import + right_track Container + Memory/Models placeholder routes
>     (the four lines Phase F swaps).
>   - `crates/ui/src/theme.rs:640-643` — `RIGHT_RAIL_WIDTH_PX = 0.0`
>     constant (K6 anchor).
>   - `crates/ui/src/theme.rs:741-750` — `SIDEBAR_GROUPS_PHASE_C`
>     Library zone (Memory + Models entries — no sidebar change).
>   - `crates/ui/src/widgets/trail_drawer.rs:12,70,175,179` —
>     trail_drawer references to `RIGHT_RAIL_WIDTH_PX` (K6 coupling;
>     stays byte-identical under Option A).
>   - `crates/ui/tests/shell_grid.rs:8,15-16` — hard invariant test
>     asserting `RIGHT_RAIL_WIDTH_PX == 0.0`; K6 Option A preserves this.
>   - `crates/ui/src/strings.rs:258-275` — `MEMORY_PLACEHOLDER` /
>     `MODELS_PLACEHOLDER` / `SIDEBAR_NAV_MEMORY` / `SIDEBAR_NAV_MODELS`
>     (deprecation precedent at `COMPARE_PLACEHOLDER:253-257`).
>   - `crates/reflection/src/store/mod.rs:27-35` — `ReflectionStore`
>     trait surface (only `upsert / top_k / count`; Q8=(b) refinement
>     in § 1.1).
>   - `crates/reflection/src/store/sqlite.rs:5-6,28-85,164-172` —
>     `SqliteReflectionStore` connection convention + `top_k` SQL
>     pattern (template for new `list_recent_lesson_cards` query).
>   - `crates/reflection/migrations/001_lesson_cards.sql:8-24` —
>     `lesson_cards` table schema (load-bearing for K1 SQL shape).
>   - `crates/reflection/src/trail_mirror.rs:8-25,164-169` —
>     hydrate-message precedent (verbatim shape for
>     `MemoryStoreReader` / `Message::MemoryHydrate`).
>   - `crates/agent/src/config.rs:323-349` —
>     `ReflectionConfig::path = ./data/audit/reflection.db` default
>     (the path the cockpit_live bin will open).
>   - `crates/ui/src/bin/cockpit_live.rs:362,743-860` — side-thread
>     tokio runtime + `audit::query::*` call pattern (template for
>     reflection + checkpoint registry hydration in Phase F).
>   - `crates/forecast/checkpoints/anchors/tcn-bs1-*.metadata.json`
>     + `tcn-bs2-*.metadata.json` — live schema source-of-truth (§ 1.2).
>   - `crates/replay-cache/src/lib.rs:8,migrations/001_replay_cache.sql` —
>     "forecast" namespace declared but **empty at 2026-05-20** (§ 1.3).
> - `bash scripts/verify_anchors.sh` re-run 2026-05-20 BEFORE this
>   pass: `ANCHORS PASS  (22 / 22)` — baseline confirmed clean.

## 1. Architect-decide resolutions

### 1.1 — K1 + Q8 resolution: Memory read-path placement (T-T1-1)

**Architect pick: refine Q8=(b) — direct SQL lives in a new
`crates/reflection/src/query.rs` module (not in `crates/ui/src/memory/store_read.rs`
as the brief named).** Reasoning:

The operator-decide Q8=(b) framing — "direct SQL in the UI module;
don't extend `ReflectionStore` trait" — was authored without
ratifying the cockpit's async/sync boundary. The reality:

- The UI crate (iced `Application`) runs on the main thread with no
  tokio runtime. Any async sqlite call in the iced `update()` arm
  would block the UI loop or require a nested runtime — both anti-
  patterns per the Phase D precedent.
- The cockpit_live bin (`crates/ui/src/bin/cockpit_live.rs:362,743+`)
  is the existing pattern for async sqlite reads: it owns a side-thread
  tokio runtime, calls `audit::query::*` synchronous helpers (which
  internally `tokio::block_on(pool.execute(...))`), and pushes results
  to the UI via a Message.
- Q8=(b)'s spirit was "don't extend the `ReflectionStore` trait" —
  honored by putting the SQL in a sibling module of `store`, not on
  the trait.

**Locked placement:**
- New module: `crates/reflection/src/query.rs` (sibling of `store/`).
  Exposes `pub fn list_recent_lesson_cards(pool: &SqlitePool, limit:
  usize) -> Pin<Box<dyn Future<Output = Result<Vec<LessonCard>,
  ReflectionStoreError>>>>` (or equivalent — architect leaves the
  developer to pick the sync vs `async fn` shape per local idiom; the
  cockpit_live bin will wrap whichever shape with `block_on`).
- New module declaration: `crates/reflection/src/lib.rs` adds
  `pub mod query;` next to `pub mod store;` (line 42-ish).
- UI side: a NEW message `Message::MemoryHydrate(Vec<LessonCardCard>)`
  in `crates/ui/src/state.rs` (sibling of `Message::TrailMirrorTick`
  Phase D precedent at `crates/ui/src/state.rs:~1425`).
- Cockpit_live wires the hydrate at boot or on first Memory screen
  open: opens `SqliteReflectionStore` against
  `./data/audit/reflection.db`, calls `reflection::query::list_recent_lesson_cards(&pool,
  N)` (N = 50 default per Q1=a + analyst R1 spec), maps `Vec<LessonCard>` →
  `Vec<LessonCardCard>` (view-model — R8.3), sends
  `Message::MemoryHydrate(cards)` to the iced `Application`.
- Fail-soft: if the reflection.db file is missing (the cold-empty path
  at 2026-05-20 — see § 1.6 below), `reflection::query` opens an empty
  store + the query returns `Ok(vec![])`, hitting the R1.4 empty-state
  placeholder.

**Locked SQL shape (`crates/reflection/src/query.rs`):**

```rust
/// Phase F (ui-rethink-phase-f-memory-models-assistant) read-path —
/// list the N most-recent lesson cards ordered by `closed_at DESC`.
///
/// Mirrors the schema declared at
/// `crates/reflection/migrations/001_lesson_cards.sql:8-24` and the
/// row materialisation at `store/sqlite.rs:233-264` (`decode_row`).
///
/// Fail-soft: returns `Ok(vec![])` if the table is empty (cold-empty
/// boot path); returns `Err(...)` only on DB connection / encoding
/// errors which the cockpit_live caller logs via `tracing::warn!` and
/// surfaces to the UI as the R1.4 empty-state placeholder.
///
/// **Q8=(b) refinement:** SQL lives in a sibling module of `store/`,
/// not on the `ReflectionStore` trait (per operator-decide). The trait
/// surface stays at 3 methods (upsert / top_k / count).
///
/// # Errors
///
/// Returns [`ReflectionStoreError::Database`] on connection failure.
pub async fn list_recent_lesson_cards(
    pool: &sqlx::SqlitePool,
    limit: usize,
) -> Result<Vec<LessonCard>, ReflectionStoreError> {
    let rows: Vec<PersistedRow> = sqlx::query_as::<_, PersistedRow>(
        "SELECT card_id, closed_at, symbol_or_pair, strategy_id, signed_pnl_usdt, \
                opening_capital_usdt, holding_period_bars, entry_regime, exit_regime, \
                outcome_class, embedding_blob, note \
         FROM lesson_cards \
         ORDER BY closed_at DESC \
         LIMIT ?",
    )
    .bind(i64::try_from(limit).unwrap_or(i64::MAX))
    .fetch_all(pool)
    .await
    .map_err(|e| ReflectionStoreError::Database(e.to_string()))?;

    rows.into_iter().map(decode_row).collect()
}
```

`PersistedRow` + `decode_row` are `pub(crate)`-promoted from
`store/sqlite.rs:89-264` — additive visibility change only (no
behavior change). Developer T-D-N3 enumerates the visibility flip.

**Rejected alternatives:**
- **Pure Q8=(b) literal — SQL inside `crates/ui/src/memory/store_read.rs`.**
  Forces a nested tokio runtime in iced `update()` OR a blocking-pool
  hack; either pattern violates Phase D's clean-async-boundary
  precedent. Rejected on architecture-edge grounds.
- **Q8=(a) — extend the `ReflectionStore` trait with `list_recent`.**
  Touches every impl (currently just `SqliteReflectionStore`); fails
  the operator's "keep trait surface minimal at v0.1.0" preference.
  Rejected per M-OD T-OD8.
- **Q8=(c) — add `list_recent` method on `SqliteReflectionStore`
  (impl only).** Half-measure that bakes the method into the impl
  struct surface; the sibling `query.rs` module is cleaner separation
  (read paths grow alongside, not bolted onto the impl). Rejected on
  layering grounds.

### 1.2 — K2 resolution: checkpoint metadata schema (T-T1-2)

**Architect pick: full serde struct shape with `#[serde(default)]` on
every non-load-bearing field.** Live schema inventoried from
`crates/forecast/checkpoints/anchors/tcn-bs1-*.metadata.json` +
`tcn-bs2-*.metadata.json` (both 852-855 bytes; identical shape).

**Live schema (verbatim from 2026-05-20 `tcn-bs1` metadata.json):**

```json
{
  "architecture": {
    "blocks": 8,
    "channels": 96,
    "dilations": [1,2,4,8,16,32,64,128],
    "dropout": "0.100000",
    "kernel": 3
  },
  "data_span": {
    "end": "2023-12-31T23:00:00Z",
    "interval": "1h",
    "source": "binance",
    "start": "2023-01-01T00:00:00Z",
    "symbols": ["ADA","AVAX","BNB","BTC","DOGE","DOT","ETH","LINK","SOL","XRP"]
  },
  "epochs_trained": 30,
  "final_train_loss": 0.000012167605746071786,
  "final_val_loss": 0.000015389239706564695,
  "model_revision": "d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2",
  "sigma_train": 10.95425033569336,
  "tokenisation": { ... },
  "training": { ... },
  "weights_sha256": "4ed9064a3871d8bc911ad8b288dccfc597caa6a09cca3b2395a9e1717b8c7025"
}
```

**Locked parser shape (`crates/ui/src/models/registry_read.rs`):**

```rust
/// Phase F — view-model populated by `discover_checkpoints` from
/// `crates/forecast/checkpoints/anchors/*.metadata.json`.
///
/// **K2 robustness contract:** every non-load-bearing field carries
/// `#[serde(default)]`. The load-bearing fields are: `model_revision`,
/// `data_span.{start,end,interval,source,symbols}`, `final_val_loss`,
/// `final_train_loss`, `sigma_train`, `weights_sha256`. Missing
/// non-load-bearing fields (e.g. `architecture.dropout` for a future
/// PatchTST family) parse to `Default::default()` — the screen renders
/// "—" for that column.
///
/// Malformed JSON (truncated file, invalid UTF-8) returns `None` from
/// the parent `parse_metadata` function; the file is skipped + a
/// `tracing::warn!` records the path. Phase F never crashes on schema
/// drift (H5).
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct CheckpointMetadata {
    pub model_revision: String,
    pub epochs_trained: u32,
    pub final_train_loss: f64,
    pub final_val_loss: f64,
    pub sigma_train: f64,
    pub weights_sha256: String,
    pub architecture: CheckpointArchitecture,
    pub data_span: CheckpointDataSpan,
    pub tokenisation: serde_json::Value,  // opaque blob — v0.1.0 doesn't render
    pub training: serde_json::Value,       // opaque blob — v0.1.0 doesn't render
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct CheckpointArchitecture {
    pub blocks: u32,
    pub channels: u32,
    pub dilations: Vec<u32>,
    pub dropout: String,
    pub kernel: u32,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct CheckpointDataSpan {
    pub start: String,
    pub end: String,
    pub interval: String,
    pub source: String,
    pub symbols: Vec<String>,
}

/// Phase F — UI view-model. Distinct from `CheckpointMetadata` to
/// avoid leaking the raw JSON shape into the UI module. R8.3.
#[derive(Debug, Clone)]
pub struct CheckpointMeta {
    pub model_revision: String,        // truncated to 8 chars at render time
    pub family: ModelFamily,           // derived from filename prefix `tcn-` / `patchtst-` / `transformer-`
    pub data_span_start: String,
    pub data_span_end: String,
    pub interval: String,
    pub symbols_count: usize,
    pub final_val_loss: f64,
    pub final_train_loss: f64,
    pub sigma_train: f64,
    pub weights_sha256: String,
    pub file_size_bytes: u64,           // from `safetensors` stat()
    pub status: ModelStatus,            // Q7=(c) — always Staged at v0.1.0
    pub source_path: std::path::PathBuf,// for "Open file" follow-up
}
```

**Family discrimination (filename prefix):**
- `tcn-bs1-...metadata.json` → `ModelFamily::Tcn`.
- `tcn-bs2-...metadata.json` → `ModelFamily::Tcn`.
- `patchtst-*` (future v2.5a) → `ModelFamily::PatchTst`.
- `transformer-*` (future v2.5b) → `ModelFamily::Transformer`.
- Unknown prefix → `tracing::warn!` + skip.

**Family enum (`crates/ui/src/models/state.rs`):**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModelFamily {
    Tcn,
    PatchTst,     // disabled at v0.1.0 (no files on disk)
    Transformer,  // disabled at v0.1.0 (no files on disk)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelStatus {
    Serving,   // v0.2.0 lifecycle (Q7=(c) deferred)
    Staged,    // v0.1.0 default for every on-disk checkpoint
    Archived,  // v0.2.0
}
```

**Five unit tests in-module (H5):**
- `parse_full_schema_round_trips` — full live `tcn-bs1` JSON parses to
  every field populated.
- `parse_missing_dropout_uses_default` — synthetic JSON with
  `architecture.dropout` removed parses with `dropout == ""`.
- `parse_missing_sigma_train_uses_default` — synthetic JSON with
  `sigma_train` removed parses with `sigma_train == 0.0`; row renders
  "—" in the sigma column.
- `parse_malformed_truncated_returns_none` — synthetic JSON truncated
  mid-key returns `None`; tracing::warn! is emitted (verify via test
  harness).
- `discover_checkpoints_skips_unknown_family` — fixture dir with one
  `tcn-*` + one `unknown-*` file returns 1 row; tracing::warn! captures
  the unknown.

### 1.3 — K3 resolution: forecast-quality sparkline data (T-T1-3)

**Architect finding: replay-cache "forecast" namespace EMPTY at
2026-05-20.** Sparkline DEFERRED to v0.2.0 per R2.2 framing.

**Citation:**
- `crates/replay-cache/migrations/001_replay_cache.sql:14-21` declares
  the `replay_cache(namespace TEXT NOT NULL)` table — the namespace
  column is the discriminator for `"forecast"` rows.
- `crates/replay-cache/src/lib.rs:8` documents the "forecast" namespace
  intent: `crates/forecast/ for DL forecaster inference caching`.
- **No populated `data/replay-cache/forecast.db` (or equivalent) exists
  on disk at 2026-05-20.** The crate is structurally ready; no writer
  has populated it during paper-trading sessions yet.
- The cockpit's `data/audit/` folder contains only `ledger.db` (135168
  bytes) — no `replay_cache*.db` siblings.

**Locked v0.1.0 sparkline behavior:**
- The Models screen row layout includes a "Forecast quality" cell —
  renders `—` (em-dash) + tooltip "Forecast quality ships when residual
  cache populates (v0.2.0)".
- No sparkline widget at v0.1.0; no residual data fetch path.
- Cell width preserved so the row layout is stable across v0.1.0 →
  v0.2.0 transition (the cell just gets a `widgets::sparkline` body in
  v0.2.0 when residuals populate).

**Rejected alternative — synthesize a residuals series from on-the-fly
inference at first model open:** scope-creep (the v2.5 TCN inference
loop is non-trivial; running it from the cockpit boot path would also
require candle feature wiring), and the result wouldn't reflect any
production "drift" signal anyway (it'd be inference over a static
fixture). Rejected.

### 1.4 — K4 + Q5 resolution: drawer-vs-Assistant-slot coexistence (T-T1-4)

**Architect pick: drawer auto-collapses the Assistant slot.**
Mutual exclusion: only one right-side surface can be open at a time.
Drawer takes priority because it carries operator-clicked context
(the Memory card chevron is a deliberate action); Assistant slot is
ambient (Phase 6 reservation). Analyst-recommended fallback Q5=(c)
(`Screen::MemoryEntry` route) NOT required — K4 conflict resolved
by mutual exclusion.

**Citation:**
- `crates/ui/src/screens/trail.rs:158,186-195` — Phase D precedent
  shows the trail drawer mounts INSIDE a `Row` next to the node-col,
  with the trail screen body owning the layout decision. Phase F
  Memory follows the same pattern: drawer renders next to the
  Memory cards list when `MemoryScreenState::drawer_open == Some(card_id)`.
- Drawer width is `RIGHT_RAIL_OPEN_WIDTH_PX = 320.0` (new K6 constant
  — see § 1.5 below). Distinct from `RIGHT_RAIL_WIDTH_PX = 0.0` which
  is preserved unchanged.
- Assistant slot is the FAR-RIGHT track (the shell's `right_track`
  Container at `shell.rs:47-49`). Memory drawer is INSIDE the screen
  body (centre column), not in the far-right track. They live in
  different shell columns.

**Re-reading § 1.4 with the layout in mind**:

```
┌─────────┬─────────────────────────────────────┬────────────┐
│ sidebar │ centre body (memory cards │ drawer) │ right_track│
│  180px  │   varies                            │ 0 or 320px │
└─────────┴─────────────────────────────────────┴────────────┘
```

The Memory drawer sits inside the centre column (the `body` arg in
`shell::view` at `:44`). The Assistant slot is the `right_track`
Container. **They're orthogonal layout-wise** — the drawer competes
for centre-body horizontal space, the Assistant slot competes for
the far-right column.

**Refined coexistence rule (no auto-collapse needed):**
- Drawer + Assistant slot CAN both be open at the same time.
- The centre body narrows when the drawer opens (cards column shrinks,
  drawer column appears next to it).
- The right_track stays at 0 or 320px based on `assistant_state.is_open`,
  independent of `memory_screen_state.drawer_open`.
- **The only K4 risk surfaces if the centre body becomes too narrow on
  small viewports.** Mitigation: layout-invariants proptest case
  `memory_drawer_open_with_assistant_open_no_zero_dim` (H6 extension)
  asserts no panic + cards column ≥ 200px + drawer ≥ 280px + far-right
  ≥ 0px under the existing 256-viewport range (320×240 → 3840×2160).
  If the proptest finds a viewport where these break, fall back to the
  auto-collapse rule (drawer forces `assistant_state.is_open = false`
  before mounting).

**Locked Q5 = (b) — Memory drawer, no fallback to (c) needed.**

### 1.5 — K6 resolution: RIGHT_RAIL_WIDTH_PX constant semantic (T-T1-5)

**Architect pick: Option A (analyst-recommended).** Keep
`RIGHT_RAIL_WIDTH_PX = 0.0` unchanged; add a NEW constant
`RIGHT_RAIL_OPEN_WIDTH_PX = 320.0` for the open-state widths (Memory
drawer + Assistant slot when `is_open == true`).

**Citation:**
- `crates/ui/src/theme.rs:643` — `pub const RIGHT_RAIL_WIDTH_PX: f32 = 0.0;`
  (unchanged).
- `crates/ui/src/shell.rs:48` — `Length::Fixed(RIGHT_RAIL_WIDTH_PX)`
  (becomes a function-of-state call, not the raw constant).
- `crates/ui/src/widgets/trail_drawer.rs:70,175,179` — three
  references to `RIGHT_RAIL_WIDTH_PX`; **all three stay byte-identical
  under Option A** (the constant value at 0.0 is unchanged; the trail
  drawer continues to render with 0-width — a Phase D placeholder
  behavior that v0.1.0 doesn't change). Phase D trail_drawer body is
  preserved verbatim (R7.2).
- `crates/ui/tests/shell_grid.rs:14-16` — hard invariant test:
  `(RIGHT_RAIL_WIDTH_PX - 0.0).abs() < f32::EPSILON`. **Option A
  preserves this test verbatim** — no test change needed.

**Locked constant additions (`crates/ui/src/theme.rs:~644`):**

```rust
/// Phase F — right-rail width when the Assistant slot is OPEN.
///
/// `RIGHT_RAIL_WIDTH_PX = 0.0` stays as the CLOSED-state default (Phase 2
/// Q7 ratification; preserved by K6 Option A for byte-identical Phase D
/// trail_drawer body + the `shell_grid.rs:14-16` hard invariant).
///
/// At Phase F, `shell::view` picks one of the two constants based on
/// `assistant_state.is_open`:
///
/// ```rust,ignore
/// let right_rail_width = if model.assistant_state.is_open {
///     RIGHT_RAIL_OPEN_WIDTH_PX
/// } else {
///     RIGHT_RAIL_WIDTH_PX  // == 0.0
/// };
/// ```
///
/// 320 px is the Lumen Phase 6 sketch width (per Lumen Phase 6 sketch
/// at `spec/design/project/ui_kits/desktop/Assistant.jsx`); also the
/// width used by the Memory drawer (Q5=(b)) so the operator's mental
/// model of "right-side panels are 320px wide" is consistent.
pub const RIGHT_RAIL_OPEN_WIDTH_PX: f32 = 320.0;
```

**Locked `shell::view` change (single hunk at `shell.rs:47-49`):**

```rust
// before:
let right_track = Container::new(Space::new())
    .width(Length::Fixed(RIGHT_RAIL_WIDTH_PX))
    .height(Length::Fill);

// after:
let right_rail_width = if model.assistant_state.is_open {
    RIGHT_RAIL_OPEN_WIDTH_PX
} else {
    RIGHT_RAIL_WIDTH_PX  // 0.0 — Phase D byte-identical default
};
let right_track = Container::new(assistant::view(&model.assistant_state, mode))
    .width(Length::Fixed(right_rail_width))
    .height(Length::Fill);
```

Note: `Space::new()` is replaced by `assistant::view(...)` even when
the slot is closed; `assistant::view` returns a 0-width
`Container<Space>` when `state.is_open == false` (byte-identical to
the existing `Space::new()` body when the right-track is 0 px wide).
This keeps the closed-state shell composition identical to today's
behavior — R7.2 surface stability honored.

### 1.6 — H1 enumeration: Memory cold-boot read budget (T-T1-6)

**Architect finding: live `data/audit/reflection.db` ABSENT at
2026-05-20.** 0-row read budget is microsecond-scale by trivial
argument; the H1 budget of < 50 ms p99 has effectively infinite
headroom.

**Method:**
- `ls -la data/audit/` returns:
  ```
  -rw-r--r--  ledger.db  135168 bytes  May 16 00:11
  ```
  **No `reflection.db` file present.** The reflection writer is
  configured (`crates/agent/src/config.rs:339` —
  `path: PathBuf::from("./data/audit/reflection.db")`) but no
  paper-trading session has yet populated lesson cards on this
  workstation.
- The cockpit's Memory screen first-open at 2026-05-20 will hit the
  R1.4 empty-state placeholder: "No memory entries yet. Memory
  populates as strategies close trades."
- When `reflection::query::list_recent_lesson_cards` is called against
  the missing file, `SqliteReflectionStore::open(path)` either
  creates an empty DB (`create_if_missing = true` at
  `store/sqlite.rs:46,51`) or fails — either way `list_recent_lesson_cards`
  returns `Ok(vec![])` and the placeholder renders.

**H1 budget conclusion: PASS by trivial argument** (0 rows ≪ 500-row
sized budget per `store/sqlite.rs:5-6`; sub-millisecond on any
hardware).

**If H1 falsifies in operator's PROD workstation (the operator has a
populated reflection.db elsewhere):** Phase D precedent at
`trail_mirror.rs` handles hydrate latency by sending the hydrate
message off the UI thread; the Memory screen renders the placeholder
until `Message::MemoryHydrate(cards)` arrives. No UI gating; K5
mitigation lifts trivially.

### 1.7 — H2 enumeration: Models cold-boot scan budget (T-T1-7)

**Architect finding: 2 × ≤ 1 KB JSON files + serde parse << 1 ms by
static argument.** H2 budget of < 50 ms p99 has ~50000× headroom.

**Method:**
- `stat -f "%z bytes" crates/forecast/checkpoints/anchors/tcn-bs1-*.metadata.json` →
  **855 bytes**.
- `stat -f "%z bytes" crates/forecast/checkpoints/anchors/tcn-bs2-*.metadata.json` →
  **852 bytes**.
- Total bytes to read + parse: ≤ 2 KB. Two file-stat() syscalls + two
  `read_to_string` + two `serde_json::from_str` calls. Memory-mapped
  page on macOS is 16 KB so both files fit in a single page read.
- `serde_json` deserializes ~100 MB/s of nested JSON on modern hardware.
  2 KB / 100 MB/s = 20 μs. ~50000× headroom over the 50 ms budget.

**H2 budget conclusion: PASS by static argument.** No micro-bench
needed.

**If H2 falsifies (operator workstation hits > 50 ms on first scan
— unlikely):** K5 mitigation lifts trivially — move the scan to
`tokio::spawn` in cockpit_live at boot; Models screen renders empty
until `Message::ModelsHydrate(checkpoints)` arrives.

### 1.8 — Net-new file count (T-T1-8 — closes R8.5)

**Architect-locked count: 11 net-new source files + 4 net-new test
files + 1 trace row flip.** Down from the analyst estimate of 8-10
because the Q4=(a) Assistant module needs 3 files (mod/state/view)
to mirror the Memory/Models module shape consistently.

| Wave | File | Net-new? |
|------|------|----------|
| A | `crates/ui/src/memory/mod.rs` | NEW |
| A | `crates/ui/src/memory/state.rs` | NEW |
| A | `crates/ui/src/memory/drawer.rs` | NEW (Q5=(b) drawer widget) |
| A | `crates/ui/src/models/mod.rs` | NEW |
| A | `crates/ui/src/models/state.rs` | NEW |
| A | `crates/ui/src/models/registry_read.rs` | NEW |
| A | `crates/ui/src/assistant/mod.rs` | NEW |
| A | `crates/ui/src/assistant/state.rs` | NEW |
| A | `crates/ui/src/assistant/view.rs` | NEW (stub-only at v0.1.0 per Q4=(a)) |
| B | `crates/reflection/src/query.rs` | NEW (§ 1.1) |
| C | `crates/ui/src/screens/memory.rs` | NEW |
| D | `crates/ui/src/screens/models.rs` | NEW |

12 net-new source files total. Plus 4 net-new test files (visual
baselines fixtures + layout invariants append + memory query unit
tests + models registry unit tests — see Wave F).

## 2. Module / file change-map

| # | File | Line(s) | Wave | Change |
|---|------|---------|------|--------|
| 1 | `crates/reflection/src/query.rs` | new | B | Per § 1.1. New module sibling of `store/`. ~50 LOC + 1 unit test (`list_recent_lesson_cards_returns_n_recent` — H4 falsification). |
| 2 | `crates/reflection/src/lib.rs` | ~42 (next to `pub mod store;`) | B | Add `pub mod query;`. ~1 LOC. |
| 3 | `crates/reflection/src/store/sqlite.rs` | 89, 233 | B | Flip `PersistedRow` + `decode_row` visibility from private to `pub(crate)` (additive — no behavior change). |
| 4 | `crates/ui/src/memory/mod.rs` | new | A | Module root: `pub mod state; pub mod drawer;`. ~10 LOC including doc. |
| 5 | `crates/ui/src/memory/state.rs` | new | A | `MemoryScreenState`, `MemoryViewMode`, `MemoryFilter`, `LessonCardCard` per R4.1 + R8.2-R8.3. ~80 LOC including doc. |
| 6 | `crates/ui/src/memory/drawer.rs` | new | D | `pub fn view(card: &LessonCardCard, mode: ThemeMode) -> Element<'_>` — Q5=(b) side drawer body. Width `RIGHT_RAIL_OPEN_WIDTH_PX`. Reuses the Phase D `trail_drawer` body composition pattern. ~80 LOC. |
| 7 | `crates/ui/src/models/mod.rs` | new | A | Module root: `pub mod state; pub mod registry_read;`. ~10 LOC. |
| 8 | `crates/ui/src/models/state.rs` | new | A | `ModelsScreenState`, `ModelFamily`, `ModelStatus`, `CheckpointMeta` per § 1.2. ~80 LOC. |
| 9 | `crates/ui/src/models/registry_read.rs` | new | B | Per § 1.2. `discover_checkpoints(dir: &Path) -> Vec<CheckpointMeta>` + `parse_metadata(path: &Path) -> Option<CheckpointMetadata>` + 5 unit tests (H5). ~140 LOC. |
| 10 | `crates/ui/src/assistant/mod.rs` | new | A | Module root: `pub mod state; pub mod view;`. ~10 LOC. |
| 11 | `crates/ui/src/assistant/state.rs` | new | A | `AssistantState { is_open: bool, mode: AssistantMode, messages: Vec<AssistantMessage> }` per R3.1. Q4=(a) — messages stays `Vec::new()` at v0.1.0; AssistantMode enum carries `Stub` only (Operator / V2Llm reserved for v0.2.0). ~50 LOC. |
| 12 | `crates/ui/src/assistant/view.rs` | new | E | `pub fn view(state: &AssistantState, mode: ThemeMode) -> Element<'_>` — when `is_open == false` returns 0-width Space (byte-identical to today's `right_track`); when `is_open == true` renders the Lumen Phase 6 stub placeholder per R3.2(a) + K7 mitigation. ~60 LOC. |
| 13 | `crates/ui/src/lib.rs` | (declarations) | A | Add `pub mod memory;`, `pub mod models;`, `pub mod assistant;` next to existing `pub mod compare;`. ~3 lines. |
| 14 | `crates/ui/src/state.rs` | ~885 (after `compare_screen_state`) | A | Add `pub memory_screen_state: MemoryScreenState`, `pub models_screen_state: ModelsScreenState`, `pub assistant_state: AssistantState` fields. Mirror in `Default` impl at `:1016` (++3 lines) + `:1116` (++3 lines) + `Debug` impl at `:965` (++3 lines). |
| 15 | `crates/ui/src/state.rs` | ~1425 (after `Message::OpenTrailFor`) | A | Add 5 new `Message` variants: `MemoryHydrate(Vec<LessonCardCard>)`, `MemoryOpenDrawer(SmolStr)` (the lesson card_id), `MemoryCloseDrawer`, `ModelsHydrate(Vec<CheckpointMeta>)`, `ToggleAssistantSlot`. Per R8.1. |
| 16 | `crates/ui/src/state.rs` | ~1380 (toolbar Messages) | A | Add 3 toolbar Messages: `MemoryToggleMode(MemoryViewMode)`, `MemorySetFilter(Option<MemoryFilter>)`, `ModelsSetFamilyFilter(Vec<ModelFamily>)`, `ModelsSetStatusFilter(Vec<ModelStatus>)`. Per R8.1. |
| 17 | `crates/ui/src/state.rs` | ~1911 (after `OpenTrailFor` arm) | A | Add 8 update-arms: `MemoryHydrate` (assign `memory_screen_state.cache`; set `last_indexed`), `MemoryOpenDrawer` (assign `memory_screen_state.drawer_open = Some(id)`), `MemoryCloseDrawer` (`drawer_open = None`), `MemoryToggleMode` / `MemorySetFilter` (assign), `ModelsHydrate` (assign + last_indexed), `ModelsSetFamilyFilter` / `ModelsSetStatusFilter` (assign), `ToggleAssistantSlot` (flip `assistant_state.is_open`). |
| 18 | `crates/ui/src/state.rs` | (#[cfg(test)] mod tests) | F | Add 3 round-trip unit tests: `memory_hydrate_populates_cache_and_indexed`, `memory_open_drawer_sets_drawer_open`, `toggle_assistant_slot_flips_is_open`. Each ~10 LOC. |
| 19 | `crates/ui/src/strings.rs` | 258-261 (Phase F deprecation) | C/D | Mark `MEMORY_PLACEHOLDER` + `MODELS_PLACEHOLDER` with `#[deprecated(since = "0.4.0", note = "Memory/Models now render real bodies — next phase removes this constant")]` per the existing `COMPARE_PLACEHOLDER` precedent at `:253-257`. |
| 20 | `crates/ui/src/strings.rs` | ~290 (Phase F section) | A | Add new constants: `MEMORY_EMPTY_STATE_COPY`, `MEMORY_DRAWER_CLOSE_LABEL`, `MEMORY_TOOLBAR_MODE_CARDS`, `MEMORY_TOOLBAR_MODE_CLUSTER`, `MEMORY_CLUSTER_DISABLED_TOOLTIP`, `MODELS_EMPTY_STATE_COPY`, `MODELS_STATUS_STAGED_TOOLTIP`, `MODELS_SPARKLINE_DEFERRED_TOOLTIP`, `MODELS_FAMILY_DISABLED_TOOLTIP`, `ASSISTANT_OFFLINE_TITLE`, `ASSISTANT_OFFLINE_BODY`, `ASSISTANT_TOGGLE_LABEL` (R3.2(a) + K7 mitigation copy). ~12 constants. |
| 21 | `crates/ui/src/theme.rs` | ~644 (after `RIGHT_RAIL_WIDTH_PX`) | A | Add `pub const RIGHT_RAIL_OPEN_WIDTH_PX: f32 = 320.0;` per § 1.5. ~15 LOC including doc-comment. |
| 22 | `crates/ui/src/screens/memory.rs` | new | C | Per R1. `pub fn view(model: &Cockpit, mode: ThemeMode) -> Element<'_>` composes: toolbar row (mode toggle + filter chip stub) + cards list + optional drawer (when `memory_screen_state.drawer_open == Some(id)`). Each card carries the right-aligned chevron emitting `Message::OpenTrailFor(audit_id)` per R6.1. ~140 LOC. |
| 23 | `crates/ui/src/screens/models.rs` | new | D | Per R2. Toolbar row (family chips + status filter) + checkpoint list (1 row per discovered file). Each row renders the columns per R2.2: model_id (truncated SHA8), family pill, training data span, val_loss, train_loss, sigma_train, weights_sha (truncated to 8 + tooltip full), file_size_bytes, status pill (always "Staged" at v0.1.0 per Q7=(c)), sparkline cell `—` placeholder. ~160 LOC. |
| 24 | `crates/ui/src/screens/mod.rs` | (declarations) | C/D | Add `pub mod memory;` + `pub mod models;` next to existing `pub mod compare;`. ~2 lines. |
| 25 | `crates/ui/src/shell.rs` | 28 | C/D/E | Extend the use-list at `crates/ui/src/screens::{...}` to include `memory, models`; extend at top-level to include `crate::assistant`. |
| 26 | `crates/ui/src/shell.rs` | 30 | E | Extend the layout-token use-list to add `RIGHT_RAIL_OPEN_WIDTH_PX`. |
| 27 | `crates/ui/src/shell.rs` | 47-49 | E | Per § 1.5. Swap `right_track` body from `Space::new()` to `assistant::view::view(&model.assistant_state, mode)` + width from raw `RIGHT_RAIL_WIDTH_PX` to function-of-state. |
| 28 | `crates/ui/src/shell.rs` | 98-99 | C/D | Swap `Screen::Memory => placeholder::view(strings::MEMORY_PLACEHOLDER, mode)` → `screens::memory::view(model, mode)` (T-D-N11 — Wave C). Swap `Screen::Models => placeholder::view(strings::MODELS_PLACEHOLDER, mode)` → `screens::models::view(model, mode)` (T-D-N14 — Wave D). |
| 29 | `crates/ui/src/bin/cockpit_live.rs` | (cockpit boot section) | B | Wire the cold-boot hydrate calls: on cockpit boot (or first Memory/Models screen open), call `reflection::query::list_recent_lesson_cards(&pool, 50)` + `ui::models::registry_read::discover_checkpoints(checkpoint_dir)` on the side-thread tokio runtime, send `Message::MemoryHydrate(cards)` + `Message::ModelsHydrate(checkpoints)` to the iced `Application`. Mirrors the Phase D trail_mirror wiring at `:362,743+`. ~40 LOC additive. |
| 30 | `crates/ui/tests/visual_snapshots.rs` (or sibling) | new fixtures | F | Add 6 `#[test] fn`s authoring `memory__cold_boot_empty`, `memory__steady_state_5_cards`, `memory__drawer_open_on_card_click`, `models__cold_boot_no_checkpoints`, `models__steady_state_2_checkpoints`, `assistant_slot__open_stub` baselines. (`assistant_slot__closed_default` is implicit — byte-identical to existing baselines per K6 Option A; no new fixture.) ~180 LOC + 6 PNG baselines. |
| 31 | `crates/ui/tests/layout_invariants.rs` | (append) | F | Add 3 proptest cases: `memory_screen_no_zero_dim`, `models_screen_no_zero_dim`, `assistant_slot_open_no_zero_dim` (H6 falsification). Each 256 viewport samples. ~50 LOC total. |
| 32 | `spec/trace.toml` | row REQ-UI-RETHINK-PHASE-F-001 | (this M-T1 pass) | Flip `state = "proposed"` → `"in-progress"`; append `decomp.md` to the `arch` array. |

**Total non-trivial files touched at developer ship:** 8 source files
modified (`crates/ui/src/lib.rs`, `state.rs`, `strings.rs`, `theme.rs`,
`shell.rs`, `screens/mod.rs`, `crates/reflection/src/lib.rs`,
`crates/reflection/src/store/sqlite.rs`, `crates/ui/src/bin/cockpit_live.rs`)
+ 12 NEW source files + 6 NEW PNG baselines + 1 trace row update.

**Net-new file count: 12** (resolves R8.5 — analyst estimated 8-10;
architect locks at 12 because Q4=(a) Assistant module needs 3 files
to mirror Memory/Models shape consistently, and the
`crates/reflection/src/query.rs` placement in § 1.1 adds one more
than the original brief anticipated).

Anchor count: 22 → 22 (additive-only by construction; R7.1-R7.7
honored).

## 3. Ordered Wave decomposition (A → E)

> Wave checklist rows are appended to `tasks.md` § "M-T1 — Architect
> decomposition" alongside the T-T1-* architect-decide rows. Each
> T-D-N row carries file:line + cargo invocation + literal expected
> output per the honest-tick rule.

### Wave A — State modules + Message variants + theme constant (R3, R4, R6, R8)

Lays the data + dispatch scaffolding. No widget code, no read-paths;
the screens in Waves C/D and the read-paths in Wave B read from this
state.

**T-D-N1** — Create `crates/ui/src/memory/{mod,state}.rs` per § 1.6 (R4.1) +
`crates/ui/src/models/{mod,state}.rs` per § 1.2 (R4.2) +
`crates/ui/src/assistant/{mod,state}.rs` per R3.1 (R4.3). Add 3
declarations to `crates/ui/src/lib.rs` (`pub mod memory; pub mod
models; pub mod assistant;`).
- Files: 6 new module files + `crates/ui/src/lib.rs` (3-line addition).
- Cargo: `cargo check -p ui`.
- Acceptance: PASS no warnings; quote literal `Checking ui v0.1.0` line.

**T-D-N2** — Add `pub const RIGHT_RAIL_OPEN_WIDTH_PX: f32 = 320.0;` to
`crates/ui/src/theme.rs:~644` per § 1.5. K6 Option A — preserve
`RIGHT_RAIL_WIDTH_PX = 0.0` unchanged.
- File: `crates/ui/src/theme.rs:~644` (15 LOC including doc-comment).
- Cargo: `cargo check -p ui` + `cargo test -p ui --test shell_grid`.
- Acceptance: PASS; quote literal `test right_rail_width_is_zero ... ok`
  line (existing test preserves byte-identically).

**T-D-N3** — Promote `PersistedRow` + `decode_row` visibility in
`crates/reflection/src/store/sqlite.rs:89,233` from private to
`pub(crate)` for `query.rs` consumption per § 1.1.
- File: `crates/reflection/src/store/sqlite.rs:89,233`.
- Cargo: `cargo check -p reflection`.
- Acceptance: PASS no warnings; quote literal `Checking reflection v0.1.0` line.

**T-D-N4** — Add new state fields to `Cockpit` at
`crates/ui/src/state.rs:~885` (after `compare_screen_state`):
`pub memory_screen_state: MemoryScreenState`, `pub models_screen_state:
ModelsScreenState`, `pub assistant_state: AssistantState`. Mirror in
3-touchpoint pattern (struct + Debug + 2× Default impls) at
`state.rs:~885,~965,~1016,~1116`.
- File: `crates/ui/src/state.rs:~885,~965,~1016,~1116`.
- Cargo: `cargo test -p ui --lib`.
- Acceptance: existing baseline test count preserved (whatever passes
  on `main` at 2026-05-20 before Phase F); quote literal `test result:
  ok. N passed; 0 failed` line.

**T-D-N5** — Add new Message variants at
`crates/ui/src/state.rs:~1380,~1425`:
`MemoryHydrate(Vec<LessonCardCard>)`, `MemoryOpenDrawer(SmolStr)`,
`MemoryCloseDrawer`, `MemoryToggleMode(MemoryViewMode)`,
`MemorySetFilter(Option<MemoryFilter>)`,
`ModelsHydrate(Vec<CheckpointMeta>)`,
`ModelsSetFamilyFilter(Vec<ModelFamily>)`,
`ModelsSetStatusFilter(Vec<ModelStatus>)`, `ToggleAssistantSlot`.
9 new variants total (R8.1).
- File: `crates/ui/src/state.rs:~1380,~1425`.
- Cargo: `cargo check -p ui`.
- Acceptance: PASS.

**T-D-N6** — Add the 9 update-arms at `crates/ui/src/state.rs:~1911`
(after `OpenTrailFor` arm). All are simple-assignment arms (no
compound dispatch — Phase F has no cross-screen seeding analogous
to Phase E's `OpenLabFromCompare`). `MemoryHydrate` + `ModelsHydrate`
also update `last_indexed`.
- File: `crates/ui/src/state.rs:~1911`.
- Cargo: `cargo check -p ui` + `cargo test -p ui --lib`.
- Acceptance: PASS; existing tests preserve.

**T-D-N7** — Add Phase F string constants per § 2 row 19/20 (12+
constants in `crates/ui/src/strings.rs:~290`; deprecate `MEMORY_PLACEHOLDER`
+ `MODELS_PLACEHOLDER` at `:258-261` per `COMPARE_PLACEHOLDER` precedent).
- File: `crates/ui/src/strings.rs:258-261,~290`.
- Cargo: `cargo check -p ui` + `cargo clippy -p ui -- -D warnings`.
- Acceptance: PASS (the deprecated constants emit warnings at their
  shell.rs:98-99 call sites until Waves C+D land — those waves swap
  the routes; warnings disappear automatically).

### Wave B — Read modules (R5.1 memory + R5.2 models)

`crates/reflection/src/query.rs` for Memory + `crates/ui/src/models/registry_read.rs`
for Models. Both populate the view-model structs from Wave A.

**T-D-N8** — Author `crates/reflection/src/query.rs` per § 1.1.
Includes `list_recent_lesson_cards(pool, limit)` + 1 unit test
(`list_recent_lesson_cards_returns_n_recent` — H4 falsification:
populate in-memory store with 5 fixture cards at known timestamps;
call `list_recent_lesson_cards(&pool, 3)`; assert the returned 3 rows
are the 3 most-recent by `closed_at DESC`). Add `pub mod query;` to
`crates/reflection/src/lib.rs:~42`.
- Files: `crates/reflection/src/query.rs` (new), `crates/reflection/src/lib.rs:~42`.
- Cargo: `cargo test -p reflection --lib query::tests`.
- Acceptance: `running 1 test` line + `test result: ok. 1 passed; 0
  failed`.

**T-D-N9** — Author `crates/ui/src/models/registry_read.rs` per § 1.2.
Includes `discover_checkpoints(dir)` + `parse_metadata(path)` +
`CheckpointMetadata` / `CheckpointArchitecture` / `CheckpointDataSpan`
serde structs + 5 unit tests (H5 falsification: full schema, missing
dropout, missing sigma_train, malformed JSON, unknown family prefix).
- File: `crates/ui/src/models/registry_read.rs` (new).
- Cargo: `cargo test -p ui --lib models::registry_read::tests`.
- Acceptance: `running 5 tests` line + `test result: ok. 5 passed; 0
  failed`.

**T-D-N10** — Wire the cold-boot hydrate calls in
`crates/ui/src/bin/cockpit_live.rs` per § 2 row 29. On cockpit boot
(or first screen open — developer picks the lighter shape), open
`SqliteReflectionStore` against `./data/audit/reflection.db` (or
config-resolved path), call `reflection::query::list_recent_lesson_cards(&pool,
50)` + `ui::models::registry_read::discover_checkpoints(checkpoint_dir)`
on the side-thread tokio runtime, send `Message::MemoryHydrate(cards)`
+ `Message::ModelsHydrate(checkpoints)` to the iced `Application`.
Mirrors `trail_mirror::TrailMirror` wiring at `:362,743+`.
- File: `crates/ui/src/bin/cockpit_live.rs:~362,~743+` (additive ~40 LOC).
- Cargo: `cargo check -p ui --bin cockpit_live --features live`.
- Acceptance: PASS no warnings; quote literal `Checking ui v0.1.0` line.

### Wave C — `screens::memory` + shell wiring (R1, R6.1)

The Memory screen body + drawer + Memory→Trail back-link.

**T-D-N11** — Author `crates/ui/src/screens/memory.rs` per § 2 row 22.
Layout: toolbar (Cards/Cluster toggle — Cluster disabled per R1.2
reserved-for-v0.2.0) + cards list (Column<MemoryCard widget>) +
optional drawer (when `memory_screen_state.drawer_open == Some(id)`).
Each card: trade-context line + lesson body (plain text per
`reflection-memory v0.1.0` Q1=Option A) + retrieval-relevance
placeholder + chevron emitting `Message::OpenTrailFor(audit_id)` (R6.1
— reuses Phase D's compound dispatch verbatim).
- Files: `crates/ui/src/screens/memory.rs` (new), `crates/ui/src/screens/mod.rs`
  (1-line `pub mod memory;` addition).
- Cargo: `cargo check -p ui`.
- Acceptance: PASS.

**T-D-N12** — Author `crates/ui/src/memory/drawer.rs` per § 2 row 6.
Side-drawer body (Q5=(b)). Width `RIGHT_RAIL_OPEN_WIDTH_PX = 320.0`
(new constant). Composition: header (close button + card title) +
body (full trade-context details + lesson body + audit-ledger
correlation row — defer the audit-query to v0.2.0 if not trivial;
v0.1.0 ships the chevron-only path). Reuses Phase D's
`widgets/trail_drawer.rs` body composition pattern verbatim.
- File: `crates/ui/src/memory/drawer.rs` (new).
- Cargo: `cargo check -p ui` + `cargo clippy -p ui -- -D warnings`.
- Acceptance: PASS.

**T-D-N13** — Swap `crates/ui/src/shell.rs:98` from
`placeholder::view(strings::MEMORY_PLACEHOLDER, mode)` to
`screens::memory::view(model, mode)`. Update the use-list at `:28` to
include `memory`.
- File: `crates/ui/src/shell.rs:28,98`.
- Cargo: `cargo test -p ui --lib` + `cargo test -p ui --test layout_invariants`.
- Acceptance: PASS; existing layout-invariants preserved.

### Wave D — `screens::models` + shell wiring (R2)

The Models screen body. No cross-link surfaces (per Q6 = (c) +
analyst R6.3 — Models cell-click is a no-op at v0.1.0).

**T-D-N14** — Author `crates/ui/src/screens/models.rs` per § 2 row 23.
Layout: toolbar (family chips — TCN enabled, PatchTST/Transformer
disabled with `MODELS_FAMILY_DISABLED_TOOLTIP`; status filter — all
defaults to Staged per Q7=(c)) + checkpoint list (Column<ModelRow
widget>). When `models_screen_state.checkpoints == Vec::new()` after
hydrate (cold-empty edge case — Q3=(a) honest placeholder), render
`MODELS_EMPTY_STATE_COPY` per R2.4. Each row renders the columns
locked in § 1.2 `CheckpointMeta` shape.
- Files: `crates/ui/src/screens/models.rs` (new), `crates/ui/src/screens/mod.rs`
  (1-line `pub mod models;` addition).
- Cargo: `cargo check -p ui`.
- Acceptance: PASS.

**T-D-N15** — Swap `crates/ui/src/shell.rs:99` from
`placeholder::view(strings::MODELS_PLACEHOLDER, mode)` to
`screens::models::view(model, mode)`. Update the use-list at `:28` to
include `models`.
- File: `crates/ui/src/shell.rs:28,99`.
- Cargo: `cargo test -p ui --lib` + `cargo test -p ui --test layout_invariants`.
- Acceptance: PASS.

### Wave E — Assistant slot wake + shell right-rail wiring (R3)

Lumen Phase 6 stub-only wake (Q4=(a)). The slot wakes structurally;
the body renders the offline-honest placeholder. v0.2.0 lifts to a
v2 LLM consumer.

**T-D-N16** — Author `crates/ui/src/assistant/view.rs` per § 2 row 12.
When `state.is_open == false` → return `Container::new(Space::new()).width(Length::Fixed(0.0))`
(byte-identical to today's `right_track` body at `shell.rs:47-49`).
When `state.is_open == true` → render the Lumen Phase 6 stub placeholder
card: `ASSISTANT_OFFLINE_TITLE` headline + `ASSISTANT_OFFLINE_BODY`
copy + a link to the v2 LLM presenter deck per K7 mitigation.
- File: `crates/ui/src/assistant/view.rs` (new).
- Cargo: `cargo check -p ui`.
- Acceptance: PASS.

**T-D-N17** — Per § 1.5 + § 2 row 27. Swap
`crates/ui/src/shell.rs:47-49` from raw `Space::new()` /
`Length::Fixed(RIGHT_RAIL_WIDTH_PX)` to the function-of-state shape:
right_rail_width picks between `RIGHT_RAIL_WIDTH_PX` (0.0) and
`RIGHT_RAIL_OPEN_WIDTH_PX` (320.0) based on `model.assistant_state.is_open`;
body becomes `assistant::view::view(&model.assistant_state, mode)`.
Update use-list at `:30` to add `RIGHT_RAIL_OPEN_WIDTH_PX`. Add
`crate::assistant` to the top-level use-list.
- File: `crates/ui/src/shell.rs:28,30,47-49`.
- Cargo: `cargo test -p ui --lib` + `cargo test -p ui --test shell_grid`
  + `cargo test -p ui --test layout_invariants`.
- Acceptance: PASS; `shell_grid::right_rail_width_is_zero` test PASSES
  (the constant stays at 0.0); layout-invariants PASSES.

### Wave F — Snapshot baselines + layout-invariants + round-trip tests (R7, H4, H5, H6)

6 NEW insta-style baselines + 3 NEW layout-invariants proptest cases
+ 3 NEW round-trip unit tests + the unit tests authored in Wave B
(T-D-N8 H4 + T-D-N9 H5). None change any of the 22 body-SHA anchors —
additive PNG files + additive proptest cases + additive `#[cfg(test)]`
tests.

**T-D-N18** — Author 6 visual snapshots in
`crates/ui/tests/visual_snapshots.rs` (or sibling): `memory__cold_boot_empty`,
`memory__steady_state_5_cards`, `memory__drawer_open_on_card_click`,
`models__cold_boot_no_checkpoints`, `models__steady_state_2_checkpoints`,
`assistant_slot__open_stub`. `assistant_slot__closed_default` is byte-identical
to existing shell baselines per K6 Option A — no new fixture needed.
- File: `crates/ui/tests/visual_snapshots.rs` + 6 sibling PNG baselines
  under `crates/ui/tests/visual-baselines/`.
- Cargo: `cargo test -p ui --test visual_snapshots`.
- Acceptance: PASS — quote literal `test result: ok. N passed; 0
  failed` line; 6 new baselines accepted on first run.

**T-D-N19** — Add 3 layout-invariants proptest cases to
`crates/ui/tests/layout_invariants.rs`: `memory_screen_no_zero_dim`,
`models_screen_no_zero_dim`, `assistant_slot_open_no_zero_dim` (H6
falsification). Each 256 viewport-size samples in the existing range
(320×240 → 3840×2160). The `assistant_slot_open_no_zero_dim` case
runs both `is_open == true` and `is_open == false` sub-cases (512
total samples — matches the analyst's H6 framing).
- File: `crates/ui/tests/layout_invariants.rs` (append).
- Cargo: `cargo test -p ui --test layout_invariants -- memory_screen_no_zero_dim
  models_screen_no_zero_dim assistant_slot_open_no_zero_dim`.
- Acceptance: `running 3 tests` + `test result: ok. 3 passed; 0
  failed`.

**T-D-N20** — Add 3 round-trip unit tests at
`crates/ui/src/state.rs:#[cfg(test)] mod tests` (after the existing
Phase E `open_lab_from_compare_*` tests): `memory_hydrate_populates_cache_and_indexed`,
`memory_open_drawer_sets_drawer_open`,
`toggle_assistant_slot_flips_is_open`.
- File: `crates/ui/src/state.rs` (append to `#[cfg(test)] mod tests`).
- Cargo: `cargo test -p ui --lib memory_hydrate_populates_cache_and_indexed
  memory_open_drawer_sets_drawer_open toggle_assistant_slot_flips_is_open`.
- Acceptance: `running 3 tests` + `test result: ok. 3 passed; 0
  failed`.

**T-D-N21** — Run `cockpit-smoke` (or the bin equivalent — confirm
via predecessor M-FINAL invocation) with `Screen::Memory`,
`Screen::Models`, and `assistant_state.is_open == true` as the
active configurations. Tester scope per M-FINAL; developer pre-runs
to confirm no panic.
- Cargo: `cargo test -p ui --test cockpit_smoke -- --nocapture`.
- Acceptance: `0 panic lines` (R7.3).

**T-D-N22** — Re-run `scripts/verify_anchors.sh` post-implementation.
Non-negotiable R7.1 carry-forward gate.
- Cargo: `bash scripts/verify_anchors.sh`.
- Acceptance: `ANCHORS PASS  (22 / 22)` literal output.

**T-D-N23** — Developer emits HANDOFF → tester envelope per AGENT.md
§ "Structured handoff envelope". Tester then runs the M-FINAL sweep
per `spec/ui-rethink-phase-f-memory-models-assistant/feature.md ##
Acceptance criteria § M-FINAL`: `cargo fmt --check`, `cargo clippy
--workspace -- -D warnings`, `cargo test --workspace --lib`, the 6
new visual snapshots (T-D-N18), the 3 new layout-invariants cases
(T-D-N19), the 3 round-trip tests (T-D-N20), H4 + H5 unit tests from
Waves B (T-D-N8, T-D-N9), `scripts/verify_anchors.sh`,
cockpit-performance v1.0.0 idle-CPU floor sweep ≤ 13.6 %, and authors
the test report.

## 4. Spike requirement

**NONE.** Phase F is purely additive UI over read paths that mirror
the Phase D trail_mirror precedent:

- **Memory read path** — `crates/reflection/src/query.rs` is one
  sqlx::query_as call against a 1-table schema (`lesson_cards`); the
  cockpit_live wiring is verbatim from `trail_mirror::TrailMirror`
  (`reflection/src/trail_mirror.rs:8-25,164-169`). Shape known.
- **Models read path** — `discover_checkpoints` is a `read_dir` + 2
  `read_to_string` + 2 `serde_json::from_str` calls against 855-byte
  files. Shape known.
- **Assistant slot Q4=(a)** — static placeholder copy + a toggle
  button; no LLM plumbing, no streaming surface, no async boundary.
- **K6** — Option A is additive constant + 3-line `shell.rs` change;
  trail_drawer body byte-identical.
- **K4** — drawer + Assistant slot live in different shell columns;
  layout-invariants proptest validates no zero-dim under any
  viewport.

If during Wave A the developer discovers a non-trivial blocker (e.g.
the `reflection::query` async-vs-sync boundary requires a different
shape, or the `cockpit_live` boot path can't be extended without
refactoring), they HANDOFF back to architect for a Wave-A spike.
**Not anticipated.**

## 5. Rollback shape (per Wave)

Each wave is independently revertible.

- **Wave A rollback:** revert the 6 new module-skeleton files (`memory/`,
  `models/`, `assistant/`) + the 3 new state fields on `Cockpit` + the
  9 new Message variants + the 12 new string constants + the 1 new
  theme constant. Cockpit reverts to v0.1.0-Phase-E shape. No
  anchor-side impact.
- **Wave B rollback:** delete `crates/reflection/src/query.rs` + the
  one-line declaration in `crates/reflection/src/lib.rs` + the
  `pub(crate)` visibility flip in `store/sqlite.rs` + the
  `crates/ui/src/models/registry_read.rs` + the cockpit_live wiring
  hunk. Wave A's data still exists but no read path consumes it;
  cockpit compiles, tests pass.
- **Wave C rollback:** revert the 2-line `shell.rs` change (use-list +
  Memory match arm) + delete `crates/ui/src/screens/memory.rs` +
  `crates/ui/src/memory/drawer.rs` + the one-line declaration in
  `screens/mod.rs`. Memory sidebar entry routes back to `placeholder::view`.
- **Wave D rollback:** symmetrical to Wave C — revert 1 line in
  `shell.rs` (Models match arm) + delete `crates/ui/src/screens/models.rs`
  + the one-line declaration in `screens/mod.rs`. Models sidebar entry
  routes back to `placeholder::view`.
- **Wave E rollback:** revert the 3-line `shell.rs` change at `:47-49`
  back to `Space::new()` / raw `RIGHT_RAIL_WIDTH_PX` + delete
  `crates/ui/src/assistant/view.rs`. Right-rail returns to 0-width
  reservation; `RIGHT_RAIL_OPEN_WIDTH_PX` constant stays but
  unreferenced (acceptable).
- **Wave F rollback:** delete the 6 new PNG baselines + their
  fixtures + the 3 new proptest cases + the 3 round-trip unit tests.
  Layout-invariants passes at the pre-Phase-F baseline; existing lib
  tests preserved.

The non-regression contract from the feature.md (22 anchors byte-
identical, lib tests PASS, layout-invariants PASS, no new external
deps) is preserved at every wave boundary by construction.

**Independently shippable per-deliverable** (per dev-note §6 line
1134): If operator review at any point splits scope — e.g. ship Memory
only, defer Models — Waves C (Memory) and D (Models) are independent
of each other; Wave E (Assistant) is independent of both. Wave A
(state) is the only shared dependency, and trimming it down (drop
unused Message variants) is a developer-side simplification.

## 6. Hard constraints honour-list

- [x] Work directly on `main` (no worktrees). Architect emits files
  only (`decomp.md`, `tasks.md` updates, `trace.toml` row flip);
  orchestrator commits. **Honored.**
- [x] iced 0.14 vendored `iced_tiny_skia` fork operator-locked
  2026-05-20. **Honored** — Phase F uses the same iced layout
  primitives (`Column<Row>`, `Button`, `Container`, `Length::Fixed`)
  already in use across Phases A-E. No iced bump.
- [x] No new external crate deps. All read paths use crates already in
  the workspace: `sqlx` (Memory) + `serde_json` (Models — already in
  `crates/forecast`). **Honored — no ADR needed.**
- [x] 22 anchored body-SHAs stay byte-identical (R7.1). **Honored by
  construction** — purely additive UI: no SQL migration on the
  anchored audit-ledger, no backtest binary change, no anchored-report
  renderer change. Architect re-ran `bash scripts/verify_anchors.sh`
  BEFORE this pass: `ANCHORS PASS  (22 / 22)` (literal output captured
  in M-T1 tick T-T1-8 below + Wave F T-D-N22).
- [x] Phase A/B/C/D/D+/E surfaces byte-identical (R7.2). **Honored**
  — Phase D trail_drawer body unchanged (K6 Option A); Phase E
  compare body unchanged; shell composition byte-identical on
  `assistant_state.is_open == false` (the default state at cockpit
  boot); sidebar IA + 3-zone grouping unchanged (`SIDEBAR_GROUPS_PHASE_C`
  preserved).
- [x] Cockpit-perf idle-CPU floor preserved (≤ 13.6 % per R7.4).
  **Verification deferred to tester at M-FINAL** per H3 (Memory +
  Models + closed Assistant slot are on-demand renders only — no new
  subscription, no new `tokio::time::interval`, matches Phase C/D/E
  precedent which all hit the budget).
- [x] Honest-tick rule — every M-T1 row carries file:line + cargo
  invocation + literal expected output (see `tasks.md` updates).
- [x] No new Lumen tokens beyond `RIGHT_RAIL_OPEN_WIDTH_PX` per R7.6.
  **Honored** — drawer/Memory/Models/Assistant bodies reuse existing
  Phase D / Phase E theme tokens (`PANEL_RAISED`, `PANEL_SUNKEN`,
  `BORDER_HAIRLINE`, `space::*`, `text::*`).

## 7. Watch recipe for long-running tasks

None of the Wave A-F tasks are individually long-running (all single-
cargo invocations completing in < 2 min). The composite
`cargo test --workspace --lib` at M-FINAL takes ≈ 3-5 min; the
tester emits the standard cockpit-smoke watch recipe at that time
(template in `spec/ui-rethink-phase-d-trail-followup/decomp.md § 7`).

If the developer kicks off a `cargo build --workspace --all-features`
to validate the cockpit_live wiring in T-D-N10 (the only build that
exercises the live feature flag), and that exceeds 2 min, emit:

```bash
watch -n 5 'cargo build -p ui --bin cockpit_live --features live 2>&1 | tail -20'
```

## 8. Handoff

Developer receives this `decomp.md` plus the appended `tasks.md`
T-D-N1..N23 checklist. Implementation order: Waves A → B → C → D → E
→ F (Wave F is mostly tester-handed; the developer's last
responsibility is T-D-N22 anchor gate + T-D-N23 handoff envelope to
tester). Waves C and D CAN run in parallel after Wave B closes (Memory
and Models screens have zero coupling); Wave E can run in parallel
with C/D after Wave A closes (Assistant slot is independent of the
Memory/Models screens — different shell column). Wave F depends on
all of C/D/E (visual snapshots exercise the wired-through shell
routes).

**M-T1 closes with:** decomp.md + tasks.md T-T1-1..T-T1-9 ticked +
trace row flipped to in-progress + anchor gate clean +
HANDOFF → developer envelope.

## Changelog

- 2026-05-20 (architect): M-T1 decomposition pass. Resolved K1 + Q8
  (refined direct-SQL placement to `crates/reflection/src/query.rs`
  honoring "no trait change" while respecting cockpit_live as the
  async/sync boundary owner), K2 (full serde struct with
  `#[serde(default)]`; live `tcn-bs1/bs2` schema inventoried), K3
  (forecast namespace empty at 2026-05-20 → sparkline deferred to
  v0.2.0; row layout ships with `—` placeholder), K4 (drawer +
  Assistant slot live in different shell columns; no auto-collapse
  needed; layout-invariants proptest validates), K6 (Option A — new
  `RIGHT_RAIL_OPEN_WIDTH_PX = 320.0`; existing `RIGHT_RAIL_WIDTH_PX =
  0.0` preserved verbatim, trail_drawer body byte-identical), H1
  (live reflection.db ABSENT at 2026-05-20 → 0-row cold-empty path
  is the dominant first-open UX; budget trivially satisfied), H2
  (2 × ≤ 1 KB JSON parse << 1 ms by static argument; ~50000×
  headroom). Wave A-F ordered with 23 T-D-N rows; net-new file count
  locked at 12 source + 6 PNG baselines + 1 trace row; spike
  requirement = NONE. Anchor baseline `ANCHORS PASS (22 / 22)` re-
  verified before this pass. Handoff envelope emitted to developer
  inline at the end of the pass.
