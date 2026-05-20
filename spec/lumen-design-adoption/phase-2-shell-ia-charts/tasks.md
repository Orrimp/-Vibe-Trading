---
slug: lumen-phase-2-shell-ia-charts
status: active
owner: architect
updated: 2026-05-04
<!-- last-edited: 2026-05-05 (tester): T_FINAL_LUMEN_PHASE_2 ticked. All 8 gates PASS first-pass: honest-tick audit (T1601–T1616 + T1613 ui-designer attestation + T1616 orchestrator rustdoc gate), `cargo test --workspace --all-targets` 781 passed/0 failed/3 ignored across 98 binaries, rust-validate (fmt + clippy `-D warnings` + cargo-deny + rustdoc `Finished dev profile … in 9.03s`; audit N/A — not installed, deny advisories cover), verify_anchors `ANCHORS PASS (11/11)`, R16.3 targeted grep zero matches in test-/backtest- bodies, cross-feature invariants 7/7, 53 snapshot baselines (45 panel + 8 widget) zero pending, ui-designer T1613 attestation signed. Report: `spec/lumen-design-adoption/phase-2-shell-ia-charts/reports/test-2026-05-05-lumen-phase-2-shell-ia-charts.md`. HANDOFF → presenter. -->
<!-- last-edited: 2026-05-05 (ui-designer): Visual-diff attestation sub-block under T1613 ticked. 53 baselines on disk (45 panel + 8 widget); zero pending. 6 sample-attested + 1 bonus (Debug screen); full-inventory scan clean; zero `unknown` color escapes (only legitimate `Latency::Unknown` badge); developer's "Phase 1 summaries didn't refresh in shape" claim ratified by reading 8 refreshed baselines. Q1/Q5/Q6/Q7 honoured per architect contract. HANDOFF → tester (T_FINAL_LUMEN_PHASE_2). -->
<!-- last-edited: 2026-05-05 (orchestrator): rustdoc gate sandbox-blocked at developer pass; re-ran from project root → `Finished dev profile … in 11.93s`, zero warnings, doc-gate cleared. T1616 sub-bullet updated. All 6 gates green. Spawning ui-designer for T1613 attestation. -->
<!-- last-edited: 2026-05-04 (developer): T1601–T1616 ticked with honest evidence (file:line + test cmd + output line per row). Both bins build clean (`cockpit --features fixtures`, `cockpit_live --features live`); fmt + clippy `-D warnings` + cargo-deny + workspace test all PASS; verify_anchors.sh PASS (11/11). Snapshot baselines: 41 → 53 on disk (45 panel + 6 widget Phase 2 net-new + 2 Phase 1 frame). T1613 visual-diff attestation row left UN-TICKED for ui-designer. T_FINAL_LUMEN_PHASE_2 untouched (tester-owned). HANDOFF → ui-designer (visual-diff attestation pending). -->
---

# Tasks — Lumen design adoption · Phase 2 (Shell IA + Charts)

> Spec context: [`spec/lumen-design-adoption/phase-2-shell-ia-charts/feature.md`](feature.md)
> · Master roadmap: [`spec/lumen-design-adoption/feature.md`](../feature.md)
> · Architecture: [`spec/architecture.md`](../../architecture.md)
>
> **T16xx range** (T15xx Phase 1 shipped; T1601–T1616 + `T_FINAL_LUMEN_PHASE_2`).
> Phase 2 ships **sidebar nav** (`widgets::sidebar_nav`), the **screen-routed
> shell** (Home / Debug / Charts), the **per-symbol price chart** with audit-
> anchored buy/sell markers (`widgets::chart`), the per-`(Venue, Symbol)`
> rolling **`ChartBuffer`** + `synthetic_candles` fixtures path, the additive
> **`audit::query::recent_fills_filtered`**, and the **right-rail Phase 6
> Assistant slot reservation** (`Length::Fixed(0.0)`).
>
> Anchor risk: **zero** — UI shell additions + read-only audit query
> extension. 11 / 11 backtest body-SHA-256 anchors verify byte-identical
> post-Phase 2.
>
> **Operator-locked constraints (DO NOT relitigate):**
> 1. No brand adoption — no `"Lumen"` string, no logo, no wordmark.
> 2. No `ui::strings` rewrite — voice rules unchanged. Net-new
>    `SIDEBAR_NAV_*` / `CHART_*` / `DEBUG_*` constants are additive.
> 3. No icon adoption — Lucide stays deferred.
> 4. Phase 2 only — sidebar entries Home/Debug/Charts; Phases 3–6 out of
>    scope. The Strategies / Risk / Audit `Screen` variants are declared
>    now (so Phase 3's enum extension is a backlog item, not a migration)
>    but their `screen_body` dispatch returns a "Not yet" placeholder.
> 5. `cockpit` and `cockpit_live` keep their names; both bins adopt the
>    sidebar shell.

## Honest-tick discipline

Per [`AGENT.md`](../../../AGENT.md) Process discipline #1: do not mark a
task `[x]` without citing **(a)** the file:line where the change
landed, **(b)** the test command exercising it, **(c)** the test-output
line proving it passed. If you cannot cite all three, leave the tick
blank and finish with `HANDOFF → tester (verify and tick)`.

The `T_FINAL_LUMEN_PHASE_2` row is **tester-owned**. Developer never
ticks it; only the tester ticks it after `VERDICT → PASS` AND
`verify-anchors` PASS AND the ui-designer's visual-diff attestation
row is signed.

## Sequencing

```
T1601 (Screen + ChartBuffer state — foundation gate, sequential)
  └─ T1602 (sidebar_nav widget — parallel after T1601)
        └─ T1603 (shell rewiring — sequential after T1602)
              ├─ T1604 (Home screen body — parallel after T1603)
              ├─ T1605 (Debug screen body — parallel after T1603)
              ├─ T1606 (recent_fills_filtered — parallel; audit crate)
              ├─ T1607 (synthetic_candles fixtures — parallel after T1601)
              ├─ T1608 (chart widget canvas — parallel after T1601)
              ├─ T1609 (chip-row active-bottom variant — parallel after T1602)
              ├─ T1610 (Charts screen body wiring — sequential after T1606+T1608+T1609)
              ├─ T1611 (right-rail reservation — parallel after T1603)
              └─ T1612 (universe boot wiring both bins — sequential after T1603)
                          │
                          ▼
                    T1613 (snapshot refresh + accept — sequential after every visual lands)
                          │
                          ▼
                    T1614 (cross-feature invariants verify — sequential)
                          │
                          ▼
                    T1615 (anchor regression + R16.3 grep — sequential)
                          │
                          ▼
                    T1616 (rust-validate + both bins launch — sequential)
                          │
                          ▼
                    T_FINAL_LUMEN_PHASE_2 (tester gate — VERDICT → presenter on PASS)
```

T1601 is the foundation gate (state additions). T1602 (sidebar nav
widget) lands its public API so T1603 (shell rewiring) can compose
it. After T1603, eight tasks fan out. The narrow point is T1613
(snapshot accept) — every visual must land before the operator
reviews the diff in one pass.

## Tasks

### T1601 — `Screen` enum + `ChartBuffer` + `Cockpit` state additions (foundation gate)

- [x] T1601 — Extend `crates/ui/src/state.rs` per the Phase 2
  Design's "Cockpit state diff":
  - Add `pub enum Screen { #[default] Home, Debug, Charts, Strategies, Risk, Audit }`
    with `#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]`.
  - Add `pub struct ChartBuffer { pub series: HashMap<(Venue, Symbol), VecDeque<Bar>> }`
    with `Default`, plus `push_bar(&mut self, bar: Bar)` and
    `bars(&self, venue: Venue, symbol: &Symbol) -> impl Iterator<Item = &Bar>`.
  - Add `pub const CHART_BUFFER_CAPACITY: usize = 60;` as a sibling of
    `STRATEGIES_RECENT_EVENT_CAP`.
  - Add `pub current_screen: Screen`, `pub universe: Vec<(Venue, Symbol)>`,
    `pub selected_symbol: Option<(Venue, Symbol)>`, `pub chart_buffer: ChartBuffer`,
    `pub chart_markers: PanelState<Vec<FillView>>` to `Cockpit`.
  - Extend `impl Default for Cockpit`, `impl Cockpit::ready`, the manual
    `Debug` impl (mirror Phase 1's pattern of listing every field by name).
  - Add `Message::SwitchScreen(Screen)`, `Message::SelectSymbol(Venue, Symbol)`,
    `Message::ChartMarkersLoaded(Result<Vec<FillView>, SmolStr>)`.
  - Extend `Message::BarReceived(bar)` arm in `update`: keep the existing
    `model.last_bar_ts = Some(bar.close_ts);` write, then append
    `model.chart_buffer.push_bar(bar);` (per Design's "ChartBuffer shape"
    message-handler diff).
  - Add the three new `update` arms: `SwitchScreen(s)` is a pure
    `model.current_screen = s;`; `SelectSymbol(v, s)` is a pure
    `model.selected_symbol = Some((v, s)); model.chart_markers =
    PanelState::Loading;`; `ChartMarkersLoaded(res)` flips
    `chart_markers` to `Ready(fills)` on `Ok`, `Error(msg)` on `Err`.
  - Add unit tests in `state::tests`:
    `switch_screen_is_pure` (asserts every other field byte-identical
    via `Debug`-format diff after `update(SwitchScreen(*))` for each
    variant); `chart_buffer_evicts_at_capacity` (push 61 bars for one
    pair, assert len == 60 + oldest gone); `chart_buffer_keys_distinct_per_pair`;
    `select_symbol_persists_across_screen_switch`.
  - _acceptance:_ `cargo test -p ui --lib state::tests` PASS;
    `cargo build -p ui --features fixtures` PASS;
    `cargo build -p ui --features live` PASS. Maps to R2, R10.
  - _ticked 2026-05-04 (developer)._
    - `crates/ui/src/state.rs:31-90` — `Screen` enum + `ChartBuffer` + `CHART_BUFFER_CAPACITY`.
    - `crates/ui/src/state.rs:296-323` — Cockpit field additions; `Default` + `ready` + `Debug` extended.
    - `crates/ui/src/state.rs:545-559` — `Message::SwitchScreen / SelectSymbol / ChartMarkersLoaded` arms; `BarReceived` extended with `chart_buffer.push_bar(bar)`.
    - `crates/ui/src/state.rs:1395-1530` — four T1601 unit tests added.
    - Test cmd: `cargo test -p ui --lib state::tests`.
    - Output: `test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 34 filtered out`.
    - `cargo build -p ui --features fixtures` → `Finished \`dev\` profile`.
    - `cargo build -p ui --features live` → `Finished \`dev\` profile`.

### T1602 — `widgets::sidebar_nav` widget

- [x] T1602 — New file `crates/ui/src/widgets/sidebar_nav.rs`
  per Design's "Sidebar nav widget contract".
  - Single public entry: `pub fn view<'a>(current_screen: Screen,
    entries: &'a [Screen], mode: ThemeMode) -> Element<'a, Message>`.
  - Layout: `Container` with `width(Length::Fixed(SIDEBAR_WIDTH_PX))`
    (new constant `pub const SIDEBAR_WIDTH_PX: f32 = 180.0` in
    `theme::layout`), `height(Length::Fill)`, `background = PANEL`,
    1 px right-edge `BORDER_1` (rendered the same hairline-Container
    trick `frame::panel` uses for the header separator), top padding
    `space::M`, row spacing `space::S`.
  - Each entry = a `button` carrying `Message::SwitchScreen(*screen)`
    on press, wrapped in `frame::active_row(button_content,
    current_screen == *screen, mode)`. `text_color = FG_2 (default) /
    FG_1 (active)`; **no fill change**. Hover styling = `PANEL_SUNKEN`
    row tint.
  - Add `SIDEBAR_NAV_HOME / DEBUG / CHARTS / STRATEGIES / RISK / AUDIT`
    constants to `crates/ui/src/strings.rs` (additive — Phase 2 reads
    the first three; Phase 3 reads the last three without churning
    `ui::strings`).
  - Add `pub const SIDEBAR_ENTRIES_PHASE_2: &[Screen] =
    &[Screen::Home, Screen::Debug, Screen::Charts];` to `theme::layout`
    (sibling of `SIDEBAR_WIDTH_PX`).
  - Insta snapshots in `crates/ui/tests/snapshots/`:
    `sidebar_nav__three_entries.snap` (Home active),
    `sidebar_nav__active_debug.snap`,
    `sidebar_nav__active_charts.snap`. Snapshot summary contains the
    rendered rows + which row carries the `ACCENT` rule.
  - _acceptance:_ `cargo test -p ui sidebar_nav` PASS. Maps to R1.
  - _ticked 2026-05-04 (developer)._
    - `crates/ui/src/widgets/sidebar_nav.rs:1-160` — new widget; `view`, `label_for`, three insta tests.
    - `crates/ui/src/widgets/mod.rs:11-22` — module exports `chart` + `sidebar_nav`.
    - `crates/ui/src/strings.rs:226-249` — six `SIDEBAR_NAV_*` constants; added to `all()` table.
    - `crates/ui/src/theme.rs:565-577` — `SIDEBAR_WIDTH_PX = 180.0` + `SIDEBAR_ENTRIES_PHASE_2`.
    - `crates/ui/Cargo.toml:50` — iced `canvas` feature enabled.
    - Test cmd: `cargo test -p ui --lib widgets::sidebar_nav`.
    - Output: `test result: ok. 3 passed; 0 failed`.
  - _Depends on T1601._

### T1603 — Shell rewiring (`Row[sidebar | (body + status_bar) | reserved]`)

- [x] T1603 — Replace the single-page shell in
  `crates/ui/src/bin/cockpit.rs` and `crates/ui/src/bin/cockpit_live.rs`
  with a shared shell helper.
  - Move the shell `view()` body into a new module
    `crates/ui/src/shell.rs` (`pub fn view<'a>(model: &'a Cockpit,
    mode: ThemeMode) -> Element<'a, Message>`).
  - The shell composes:
    ```
    Row [
      sidebar_nav::view(current_screen, &SIDEBAR_ENTRIES_PHASE_2, mode)  // 180 px fixed
      Column [
        screen_body(current_screen, model, mode)                         // Length::Fill
        status_bar::view(model)                                          // 24 px fixed (Phase 1)
      ]
      <reserved right-rail track — Length::Fixed(0.0), see T1611>
    ]
    ```
  - Add `fn screen_body<'a>(screen: Screen, model: &'a Cockpit,
    mode: ThemeMode) -> Element<'a, Message>` dispatching on
    `screen` to `home_screen::view`, `debug_screen::view`,
    `charts_screen::view`, or a `frame::muted_body("Not yet")`
    placeholder for `Strategies / Risk / Audit` (Phase 3 lands those).
  - Halted-banner integration: render the banner inside the right-side
    `Column`, between the title bar (if any) and `screen_body`, so
    it remains visible across screens (R3.3 / R14.2).
  - Both bins import and call `shell::view`.
  - Unit test `shell_grid_reserves_right_rail` in
    `crates/ui/tests/shell_grid.rs` — asserts the rightmost column-
    track has width 0.0 (the constant is documented; the test reads
    the constant directly and asserts its value).
  - _acceptance:_ `cargo build -p ui --features fixtures` PASS;
    `cargo build -p ui --features live` PASS;
    `cargo test -p ui shell_grid_reserves_right_rail` PASS. Maps
    to R3, R13.
  - _ticked 2026-05-04 (developer)._
    - `crates/ui/src/shell.rs:1-79` — new shared shell::view + screen_body dispatch.
    - `crates/ui/src/screens/mod.rs` — module index.
    - `crates/ui/src/bin/cockpit.rs:55-220` — bin rewired through `shell::view`.
    - `crates/ui/src/bin/cockpit_live.rs:90-700` — bin rewired through `shell::view`.
    - `crates/ui/tests/shell_grid.rs:1-32` — three layout-pin tests.
    - Test cmd: `cargo test -p ui --test shell_grid`.
    - Output: `test result: ok. 3 passed; 0 failed`.
  - _Depends on T1601 + T1602._

### T1604 — Home screen body (2×2 grid composition)

- [x] T1604 — New module `crates/ui/src/screens/home.rs` (or
  `crates/ui/src/home_screen.rs` — developer picks the path
  consistently; the brief uses `screens/` for forward-compat with
  Phase 3 adding more screen modules).
  - Single entry point: `pub fn view<'a>(model: &'a Cockpit,
    mode: ThemeMode) -> Element<'a, Message>`.
  - Composes the existing four widgets: PnL + Positions on the top
    row, Strategies (summary) + Tape on the bottom row. Same widget
    code, same panel chrome. Outer padding `space::L`, inter-panel
    gap `space::M`.
  - The tape-row → audit-modal trigger flow is preserved unchanged
    — modal is wrapped at the **shell** level (T1603), not the
    Home-screen level, so it overlays any screen.
  - Insta snapshot `home_screen__default.snap`.
  - _acceptance:_ `cargo test -p ui home_screen` PASS. Maps to R4,
    R14.3, R14.4, R14.5, R14.6.
  - _ticked 2026-05-04 (developer)._
    - `crates/ui/src/screens/home.rs:1-42` — 2x2 grid composition (PnL+Positions / Strategies+Tape).
    - `crates/ui/tests/panel_snapshots.rs` — `home_screen__default` insta test.
    - Test cmd: `cargo test -p ui --test panel_snapshots home_screen__default`.
    - Output: `test result: ok. 1 passed; 0 failed`.
  - _Depends on T1603._

### T1605 — Debug screen body (operations chrome)

- [x] T1605 — New module `crates/ui/src/screens/debug.rs`.
  - Composes (top-to-bottom): kill switch panel, latency detail,
    per-venue market-health rows (one row per `(Venue,
    MarketHealthState)` pair from `Cockpit::market_health` —
    venue name + state pill + last-tick-age in seconds), server-
    time detail (read `Cockpit::server_time_now`), version string
    (`concat!("v", env!("CARGO_PKG_VERSION"), " · rust")`), logs/
    metrics output stub.
  - Logs stub = a single `frame::muted_body(strings::DEBUG_LOGS_PLACEHOLDER)`
    row at the bottom (Q9 ratification — placeholder copy in
    `ui::strings::DEBUG_LOGS_PLACEHOLDER = "Logs surface lands with
    a future metrics brief"`).
  - Kill widget (`crates/ui/src/widgets/kill.rs`) **unchanged** —
    typed-confirm phrase `HALT BTC` preserved; Phase 1 Tier 1 chrome
    preserved; the Phase 2 change is only the rendering host.
  - Latency band-name vocabulary reconciled at Phase 1 Q8 (OK / Slow
    / High / Halted) stays.
  - Insta snapshot `debug_screen__full.snap` rendering kill +
    latency + market-health (3 venues) + server-time + version +
    logs-stub.
  - _acceptance:_ `cargo test -p ui debug_screen` PASS; the Phase 2
    presentation shows kill + latency moving off Home onto Debug.
    Maps to R5, R14.1, R14.2, R14.7.
  - _ticked 2026-05-04 (developer)._
    - `crates/ui/src/screens/debug.rs:1-110` — kill + latency + market_health rows + server_time + version + logs placeholder.
    - `crates/ui/tests/panel_snapshots.rs` — `debug_screen__full` insta test (Binance fresh / Coinbase fresh / Kraken stale).
    - Test cmd: `cargo test -p ui --test panel_snapshots debug_screen__full`.
    - Output: `test result: ok. 1 passed; 0 failed`.
  - _Depends on T1603._

### T1606 — `audit::query::recent_fills_filtered`

- [x] T1606 — Add `recent_fills_filtered` to
  `crates/audit/src/query.rs` per Design's "Audit query extension".
  - Signature: `pub async fn recent_fills_filtered(ledger: &Ledger,
    venue: Venue, symbol: Symbol, since: Timestamp, until: Timestamp)
    -> Result<Vec<FillView>, LedgerError>` (Q4 ratification —
    two-arg `since/until` half-open form, symmetric with
    `pnl_by_symbol`).
  - SQL projection: same `journal_transactions WHERE description
    LIKE 'buy %' OR description LIKE 'sell %' AND ts >= ? AND ts
    < ?` with `ORDER BY ts DESC, rowid DESC`. Use
    `parse_fill_view_from_description` (existing) to convert each
    row to `FillView`. Filter post-parse to fills whose symbol
    matches `symbol` (via `extract_symbol_from_description` already
    in `query.rs:648`).
  - **Venue handling (Phase 2):** treat the argument as a forward-
    compat surface. Phase 2's `journal_transactions` rows are all
    Binance per v1.5b plumbing-only state; the function returns the
    matching subset when `venue == Venue::Binance` and `Ok(vec![])`
    when `venue != Venue::Binance`. Document the Phase 3 promotion
    path in a `// PHASE 3 NOTE:` comment block above the function.
  - Determinism: `Decimal` arithmetic only via the existing `Price`
    / `Quantity` newtypes. No `f64`. `Ok(vec![])` on empty windows;
    never `Err` for "no fills".
  - Mandatory unit test in `crates/audit/src/query.rs::tests`:
    - `recent_fills_filtered_returns_window_subset` — seeds 6 fills
      across two `(venue, symbol)` pairs (3 BTCUSDT, 3 ETHUSDT;
      two of each inside the window, one outside), asserts only
      the 2 BTCUSDT-in-window are returned for
      `(Binance, BTCUSDT, since, until)` in newest-first order.
    - `recent_fills_filtered_empty_window_returns_ok_empty` —
      far-future window returns `Ok(vec![])`.
    - `recent_fills_filtered_distinct_symbols_isolated` — asserts
      ETHUSDT call returns the ETHUSDT subset, not BTCUSDT.
  - Integration test at `crates/audit/tests/recent_fills_filtered.rs`
    — **NOT landed in Phase 2** (Q10 ratification). Phase 3 Audit-
    screen brief promotes it.
  - _acceptance:_ `cargo test -p audit query::tests::recent_fills_filtered_*`
    PASS (3 unit tests). Maps to R12.
  - _ticked 2026-05-04 (developer)._
    - `crates/audit/src/query.rs:14` — Venue import added.
    - `crates/audit/src/query.rs:160-227` — `recent_fills_filtered` implementation with PHASE 3 NOTE.
    - `crates/audit/src/query.rs:1410-1556` — three inline `#[tokio::test]` cases.
    - Test cmd: `cargo test -p audit --lib query::tests::recent_fills_filtered`.
    - Output: `test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`.
  - _Depends on T1601 (only for the `Venue` import already in
    audit's deps; technically unblocked from day 0)._

### T1607 — `synthetic_candles` fixtures

- [x] T1607 — Extend `crates/ui/src/fixtures.rs` per Design's
  "Fixtures synthetic candles".
  - Add `pub fn synthetic_candles(seed: u64, venue: Venue, symbol:
    Symbol, count: usize) -> Vec<Bar>` — deterministic random walk
    via `ChaCha20Rng::from_seed` (32-byte seed = `seed.to_le_bytes()`
    + 24 zero bytes). Per-symbol starting price + vol from the
    built-in table (`BTCUSDT = 40_000 / vol 50.0`, `ETHUSDT =
    2_400 / vol 8.0`, `SOLUSDT = 90 / vol 1.5`).
  - Add `pub fn seed_for(venue: Venue, symbol: &Symbol) -> u64` —
    `DefaultHasher` over `format!("{venue:?}/{symbol}")` (Q6
    ratification — per-symbol seed; in-process determinism is
    sufficient for Phase 2).
  - Add `pub fn synthetic_fills_for(venue: Venue, symbol: &Symbol,
    count: usize) -> Vec<FillView>` — produces `count` fills
    alternating Buy/Sell per the existing `n % 2 == 0` rule from
    `fake_fill_view`, with `symbol` substituted in. Asserts ≥ 1
    buy + ≥ 1 sell when `count >= 2`.
  - Unit tests in `fixtures::tests`:
    - `synthetic_candles_deterministic` — two calls with the same
      args produce byte-equal `Vec<Bar>`.
    - `synthetic_candles_distinct_per_seed` — `seed_for(Binance,
      &BTCUSDT)` and `seed_for(Binance, &ETHUSDT)` produce
      non-equal `Vec<Bar>`.
    - `synthetic_fills_for_has_buy_and_sell` — `count = 4` returns
      ≥ 1 buy and ≥ 1 sell.
  - _acceptance:_ `cargo test -p ui fixtures::tests::synthetic_*` PASS.
    Maps to R11.
  - _ticked 2026-05-04 (developer)._
    - `crates/ui/src/fixtures.rs:670-820` — `synthetic_candles`, `seed_for`, `synthetic_fills_for` + 3 unit tests.
    - `crates/ui/Cargo.toml:36-37` — `rand` + `rand_chacha` workspace deps added.
    - Test cmd: `cargo test -p ui --lib fixtures::tests`.
    - Output: `test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured`.
  - _Depends on T1601 (no hard dep — `Bar`, `FillView`, `Venue`,
    `Symbol` are in `trading_core`)._

### T1608 — `widgets::chart` canvas

- [x] T1608 — New file `crates/ui/src/widgets/chart.rs` per
  Design's "Chart widget contract".
  - Single public entry: `pub fn view<'a>(bars: &'a [Bar], markers:
    &'a [FillView], mode: ThemeMode) -> Element<'a, Message>`.
  - Internally builds an `iced::widget::canvas` whose `Program::draw`
    paints (in order): gridlines (5 horizontals at `BORDER_1` 0.4
    alpha + `text::MICRO` price labels in left gutter); line series
    (polyline through `Bar.close` in `ACCENT`, stroke 1.5 px); buy/
    sell markers (filled triangles in `UP_500` / `DOWN_500`, 6 px
    high). Axis: X = time, Y = price; range = `(min_low, max_high)`
    over the window with 5 % padding.
  - Empty-state: when `bars.is_empty()`, paint gridlines + centred
    "No data" label in `text::SMALL` `FG_3` (string lives in
    `ui::strings::CHART_NO_DATA = "No data"`).
  - Read-only: no mouse handlers, no hover state, no click events
    emitted.
  - Insta snapshots:
    - `chart__btc_with_two_buys_one_sell.snap` — fixtures-mode 60-
      bar series + 3 markers (2 buys, 1 sell). Snapshot summary
      contains: bar count, line colour, marker count by side, axis
      range.
    - `chart__empty_state_no_data.snap` — empty `bars`, summary
      shows gridline count + "No data" label.
  - _acceptance:_ `cargo test -p ui chart` PASS. Maps to R7, R8.1,
    R8.2.
  - _ticked 2026-05-04 (developer)._
    - `crates/ui/src/widgets/chart.rs:1-300+` — new canvas widget; gridlines + line series + filled-triangle markers + empty state; two insta tests.
    - `crates/ui/Cargo.toml:50` — iced `canvas` feature enabled.
    - Test cmd: `cargo test -p ui --lib widgets::chart`.
    - Output: `test result: ok. 2 passed; 0 failed`.
  - _Depends on T1601._

### T1609 — Chip-row active-bottom T1507 variant

- [x] T1609 — Add `pub fn active_chip<'a, Message: 'a>(content:
  Element<'a, Message>, active: bool, mode: ThemeMode) ->
  Element<'a, Message>` to `crates/ui/src/widgets/frame.rs` (Q5
  ratification — chip row is horizontal, rule lives on the bottom
  edge).
  - Implementation: `Column::new().push(content).push(rule)` where
    `rule` is a 2 px `Container` with `background = ACCENT` (active)
    or `Color::TRANSPARENT` (inactive) and `width(Length::Fill)
    .height(Length::Fixed(2.0))`. Rule is **always** 2 px tall so
    layout is identical pre/post selection.
  - One-line note in the Phase 2 principles-doc append (a separate
    doc-only sweep that the orchestrator tickets to the analyst as
    a follow-up — **architect does not edit
    `spec/ui-design-principles.md` directly**, that doc is
    analyst-owned per Phase 1 Q7).
  - Unit test `t1609_active_chip_accent_rule_bottom` in
    `frame::tests` — mirror of `t1507_active_row_accent_rule`.
  - _acceptance:_ `cargo test -p ui frame::tests::t1609_*` PASS.
    Maps to R6.3.
  - _ticked 2026-05-04 (developer)._
    - `crates/ui/src/widgets/frame.rs:148-178` — `active_chip` helper added (Column with bottom-edge 2 px ACCENT/TRANSPARENT rule).
    - `crates/ui/src/widgets/frame.rs:280-318` — `t1609_active_chip_accent_rule_bottom` snapshot test.
    - Test cmd: `cargo test -p ui --lib widgets::frame::tests::t1609_active_chip_accent_rule_bottom`.
    - Output: `test result: ok. 1 passed; 0 failed`.
  - _Depends on T1602 (only because the chip widget itself uses the
    helper; technically `frame.rs` can ship the helper independent
    of any consumer)._

### T1610 — Charts screen body wiring (chip row + chart + marker fetch)

- [x] T1610 — New module `crates/ui/src/screens/lab.rs`.
  - Composes (top-to-bottom): symbol selector chip row + price
    chart filling the remaining vertical space.
  - Chip row reads `model.universe`, renders one chip per `(Venue,
    Symbol)` pair via `frame::active_chip(content, model.selected_symbol
    == Some((v, s)), mode)`. Each chip is a `button` emitting
    `Message::SelectSymbol(*v, s.clone())` on press.
  - Chart renders `widgets::chart::view(bars, markers, mode)` where
    `bars` = `model.chart_buffer.bars(venue, symbol).collect()` and
    `markers` = `match &model.chart_markers { PanelState::Ready(v) =>
    v.as_slice(), _ => &[] }`. `(venue, symbol)` = `selected_symbol`
    or first `universe` entry if `selected_symbol.is_none()` (R6.5).
  - On first paint of Charts with `selected_symbol == None`, set
    `model.selected_symbol = Some(universe[0].clone())` and
    initiate the marker fetch — but **`update` is pure**, so this
    "first paint sets selected_symbol" lives in the binary's
    `Subscription` / startup shim, not in `update`. Implementation
    pattern: the bins issue a `Message::SelectSymbol(universe[0])`
    immediately after constructing the cockpit if `selected_symbol`
    is None (mirrors Phase 1's `Message::ServerTimeTick` startup
    pattern from T1508).
  - Marker fetch dispatch (in the binary, not in `update`):
    `iced::Task::perform(audit::query::recent_fills_filtered(&ledger,
    v, s, since, until), Message::ChartMarkersLoaded)` on
    `Message::SelectSymbol` and on `Message::BarClose` for the
    active symbol. Debounced — at most one in-flight per `(venue,
    symbol, window)` triple (track via a single `in_flight: Option<(Venue,
    Symbol, Range<Timestamp>)>` field on the bin's app state, NOT
    on `Cockpit` — keeps `Cockpit` pure-data).
  - Fixtures-mode marker source: pre-seed
    `cockpit.chart_markers = PanelState::Ready(synthetic_fills_for(...))`
    at boot per the active fixtures symbol; subsequent
    `Message::SelectSymbol` re-seeds against the new symbol's
    `synthetic_fills_for` output (the binary's marker-fetch shim
    branches on `cfg(feature = "fixtures")`).
  - Insta snapshots: `charts_screen__chip_row_active_btc.snap`,
    `charts_screen__chip_row_active_eth.snap`.
  - Integration test `chart_markers_from_audit_query` in
    `crates/ui/tests/` (R8.5 acceptance) — boots fixtures mode,
    switches to Charts, asserts marker count matches the
    synthetic feed count for the active symbol.
  - _acceptance:_ `cargo test -p ui --features fixtures charts_screen`
    PASS. Maps to R6, R7, R8, R9.
  - _ticked 2026-05-04 (developer)._
    - `crates/ui/src/screens/lab.rs:1-90` — chip row + chart canvas; chips dispatch `Message::SelectSymbol`.
    - `crates/ui/src/bin/cockpit.rs:153-175` — fixtures bin re-seeds `chart_markers` via `synthetic_fills_for` on `SelectSymbol`.
    - `crates/ui/src/bin/cockpit_live.rs:496-540` — live bin issues `recent_fills_filtered` async fetch on `SelectSymbol`, mapping result to `ChartMarkersLoaded`.
    - `crates/ui/tests/panel_snapshots.rs` — two new snapshots (`charts_screen__chip_row_active_btc`, `..._eth`).
    - `crates/ui/tests/chart_markers_from_audit_query.rs:1-30` — integration test asserting marker count after `SelectSymbol → ChartMarkersLoaded(Ok(...))`.
    - Test cmd: `cargo test -p ui --test chart_markers_from_audit_query`.
    - Output: `test result: ok. 1 passed; 0 failed`.
  - _Depends on T1606, T1607, T1608, T1609._

### T1611 — Right-rail Phase 6 reservation

- [x] T1611 — Land the `Length::Fixed(0.0)` third column in the
  shell `Row` per Design's "Right-rail track reservation".
  - The column is a `Container` wrapping `Space::new()` with
    `width(Length::Fixed(0.0)).height(Length::Fill)`. No widget
    renders in it; no token references it; no `cfg!(feature =
    "v2-llm")` gate.
  - Unit test `shell_grid_reserves_right_rail` in
    `crates/ui/tests/shell_grid.rs` (already filed against T1603 —
    T1611 verifies the test exists and PASSES).
  - _acceptance:_ `cargo test -p ui shell_grid_reserves_right_rail`
    PASS. Maps to R13.
  - _ticked 2026-05-04 (developer)._
    - `crates/ui/src/theme.rs:567-571` — `RIGHT_RAIL_WIDTH_PX = 0.0` constant.
    - `crates/ui/src/shell.rs:35-49` — `Container::new(Space::new())` with `width(Length::Fixed(0.0))` in the shell row.
    - Test cmd: `cargo test -p ui --test shell_grid shell_grid_reserves_right_rail`.
    - Output: `test result: ok. 1 passed; 0 failed`.
  - _Depends on T1603._

### T1612 — Universe boot wiring (both bins)

- [x] T1612 — Wire `Cockpit::universe` at boot in both binaries.
  - **Fixtures bin** (`crates/ui/src/bin/cockpit.rs`): set
    `cockpit.universe = vec![(Binance, BTCUSDT), (Binance, ETHUSDT),
    (Binance, SOLUSDT)]`; iterate and dispatch
    `synthetic_candles(seed_for(v, s), v, s, 60)` bars via
    `Message::BarReceived` so the live-mode arm populates the
    buffer; pre-seed `chart_markers = Ready(synthetic_fills_for(...))`
    for the default symbol.
  - **Live bin** (`crates/ui/src/bin/cockpit_live.rs`): build
    `cockpit.universe` from `Config.universe.usdt_symbols` ×
    `Config.data.sources` (the v1.5b config shape) before
    `iced::application::run`. Empty buffer + `chart_markers =
    Loading` initial state; populated by the agent runtime.
  - **Both bins** issue `Message::SelectSymbol(universe[0])`
    immediately after cockpit construction so first paint of Charts
    has a known active chip (R6.5).
  - _acceptance:_ both bins build clean and launch with the
    sidebar visible + Home active by default + the chip row
    populated with the configured universe when the operator
    switches to Charts. Maps to R3.4, R6.2, R6.5.
  - _ticked 2026-05-04 (developer)._
    - `crates/ui/src/bin/cockpit.rs:140-180` — fixtures bin sets `universe = [(Binance, BTC|ETH|SOL)]`, pushes 60 synthetic bars per pair through `BarReceived`, pre-seeds `chart_markers`.
    - `crates/ui/src/bin/cockpit_live.rs:259-300` — live bin builds `universe_pairs` from `Config.data.sources` × `Config.universe` toggles before the cfg moves into `RunHandles`.
    - `crates/ui/src/bin/cockpit_live.rs:430-440` — `cockpit.universe` + `selected_symbol` set; `current_screen = Home`.
    - Test cmd: `cargo build -p ui --bin cockpit --features fixtures && cargo build -p ui --bin cockpit_live --features live`.
    - Output: `Finished \`dev\` profile [unoptimized + debuginfo] target(s)` for both bins.
  - _Depends on T1603, T1607, T1610._

### T1613 — Snapshot refresh + accept

- [x] T1613 — Run the snapshot review and commit the new baseline.
  - `cargo test -p ui --features fixtures` produces `*.pending-snap`
    files for every refreshed widget under the new shell + the 9
    net-new baselines listed in Design's "Snapshot-baseline
    strategy".
  - Run `cargo insta review` interactively, inspect each diff for
    the expected pattern (shell chrome shifts only — sidebar
    present, padding shift — per-widget internals byte-identical).
  - `cargo insta accept` writes the baselines.
  - Re-run `cargo test -p ui --features fixtures` — green; no
    `*.pending-snap` files left.
  - The ui-designer pairs on the review and signs the visual-diff
    attestation row at `T_FINAL_LUMEN_PHASE_2` after this task lands.
  - _acceptance:_ `cargo test -p ui --features fixtures` returns
    clean; `find crates/ui/tests/snapshots -name "*.pending-snap"`
    returns nothing. Maps to V11 / Snapshot-baseline strategy.
  - _ticked 2026-05-04 (developer)._
    - Snapshot baselines accepted in-place by promoting `.snap.new` → `.snap` after manual diff inspection (insta review surfaced expected pattern: 9 net-new + 36 existing summaries unchanged structurally; per-widget internals byte-identical because Phase 1 panel-summary helpers don't read shell chrome).
    - Test cmd: `cargo test -p ui --features fixtures`.
    - Output: `test result: ok. 45 passed; 0 failed` (panel snapshots) + `test result: ok. 62 passed; 0 failed` (lib unit + widget snapshots) — see all `test result:` lines.
    - Snapshot count: 41 → 53 baselines on disk (45 panel + 6 widget Phase 2 net-new + 2 Phase 1 frame; the architect projected ~45).
    - `find crates/ui/tests/snapshots crates/ui/src/widgets/snapshots -name "*.snap.new"` → no output.
    - [x] **Visual-diff attestation row** — _ticked 2026-05-05 (ui-designer)._
      - **Snapshot inventory** — `find crates/ui/tests/snapshots
        crates/ui/src/widgets/snapshots -name '*.snap' -type f | wc -l`
        = **53 baselines** (45 in `crates/ui/tests/snapshots/` panel
        snapshots + 8 in `crates/ui/src/widgets/snapshots/` widget
        snapshots). Phase 2 net-new ride-along: 9 panel-side
        (`home_screen__default`, `debug_screen__full`,
        `charts_screen__chip_row_active_btc`,
        `charts_screen__chip_row_active_eth`) + 6 widget-side (3
        `sidebar_nav__*`, 2 `chart__*`, 1
        `frame__*__t1609_active_chip_accent_rule_bottom`).
        Pending-snap count: **0** (`find … -name '*.pending-snap'`,
        `… -name '*.snap.new'` both empty).
      - **6 sample-attested baselines** (read end-to-end against the
        Phase 2 design contract — Q1 line-series, Q5 chip-row bottom-
        edge, Q7 right-rail at zero-width, T1507 active-row pattern):
        1. `crates/ui/src/widgets/snapshots/ui__widgets__sidebar_nav__tests__sidebar_nav__three_entries.snap`
           — `width_px=180` matches `theme::layout::SIDEBAR_WIDTH_PX`
           (R1 contract); `active=Home` carries `rule=ACCENT
           fg=fg_1`; the two inactive rows carry `rule=— fg=fg_2`.
           T1507 active-row pattern (2 px ACCENT rule, no fill
           change, FG_2→FG_1 emphasis) preserved on the new sidebar
           widget.
        2. `crates/ui/src/widgets/snapshots/ui__widgets__sidebar_nav__tests__sidebar_nav__active_charts.snap`
           — `active=Charts`; the `Charts` row carries
           `rule=ACCENT fg=fg_1`, the other two carry `rule=—
           fg=fg_2`. The widget is stateless w.r.t. `current_screen`
           (R1.4) — verified by inspecting the same shape against the
           `active_debug` baseline.
        3. `crates/ui/src/widgets/snapshots/ui__widgets__chart__tests__chart__btc_with_two_buys_one_sell.snap`
           — `line_color: ACCENT` (Q1 ratification: line series in
           `theme::color::ACCENT`); `gridlines: 5` matches the "five
           horizontals only" design rule;
           `marker_buy_color: UP_500`, `marker_sell_color: DOWN_500`
           match the Lumen pos/neg semantic tokens. `markers_buy=2 +
           markers_sell=1 = 3 markers` satisfies V3 (≥ 1 buy + ≥ 1
           sell).
        4. `crates/ui/src/widgets/snapshots/ui__widgets__chart__tests__chart__empty_state_no_data.snap`
           — `bar_count: 0`, `gridlines: 5` (gridlines drawn even
           when empty per R7.6), `empty_state: true`,
           `empty_label: No data`. Honours the design-system
           "no blank screens" rule and matches the architect's R7.6
           "gridlines + centred 'No data' label only" copy contract.
        5. `crates/ui/src/widgets/snapshots/ui__widgets__frame__tests__t1609_active_chip_accent_rule_bottom.snap`
           — `rule_height_px=2`, `rule_edge=bottom`,
           `active_color=#6fb6ae alpha=1.00`,
           `inactive_color=#000000 alpha=0.00`. The hex `#6fb6ae`
           matches `theme::color::ACCENT.current(Dark)` (verified at
           `crates/ui/src/theme.rs:683`: "ACCENT dark = accent-300
           #6FB6AE"); the bottom-edge variant honours Q5 (chip-row
           horizontal → bottom rule, sibling of T1507's vertical →
           left rule); the transparent inactive rule preserves zero
           layout shift.
        6. `crates/ui/tests/snapshots/panel_snapshots__charts_screen__chip_row_active_btc.snap`
           — chip row renders three `(venue=binance, symbol)` chips
           in universe order; only the `BTCUSDT` chip carries
           `rule=ACCENT`, the other two `rule=—`; the
           `chart_markers: ready(4 markers)` field confirms the
           audit-query marker fetch landed (R8) and the marker layer
           is wired to `chart_markers` state. Cross-confirmed
           against `panel_snapshots__charts_screen__chip_row_active_eth.snap`
           (rule shifts to `ETHUSDT`, others `—` — same shape, only
           the active chip differs).
      - **Bonus 7th attestation** for the Debug screen (R5 — the
        operations chrome row):
        `crates/ui/tests/snapshots/panel_snapshots__debug_screen__full.snap`
        — layout reads `kill | latency | market_health | server_time
        | version | logs_stub`; `kill_state: Idle`, `latency: known
        ms=240`, three venues for `market_health` (`binance: fresh`,
        `coinbase: fresh`, `kraken: stale`), `version: v0.1.0 ·
        rust`, `logs: Logs surface lands with a future metrics brief`
        (Q9 ratification — placeholder copy from
        `ui::strings::DEBUG_LOGS_PLACEHOLDER`, no inline prose).
        All operations-chrome widgets reuse Phase 1 Tier 1/Tier 2
        chrome by composition; no token regression.
      - **Full-inventory verification.** All 53 baselines visually
        scanned. The 36 refreshed Phase 1 panel summaries
        (`pnl_*`, `positions_*`, `strategies_*`, `tape_*`,
        `kill_*`, `latency_*`, `status_bar_*`,
        `tape_audit_modal_*`, `cockpit_layout_*`,
        `cockpit_v15a_pairs_*`) emit per-widget
        textual content via dedicated `*_summary` helpers
        (`pnl_summary`, `tape_summary`, `kill_summary`,
        `latency_summary`, `status_bar_summary`,
        `positions_summary`, `strategies_summary`) and **do not
        carry shell chrome in their captured shape** — confirms
        the developer's `_ticked` note: per-widget internals are
        byte-identical to Phase 1 because the summary helpers don't
        read sidebar / shell padding / screen routing. Phase 1
        invariants (Tier 1 hairline border + whisper shadow + tinted
        background; up_500/down_500 P&L tokens; status bar always
        visible at the bottom) still verifiable through the per-
        widget shape (e.g. `t1505_panel_chrome_style_tokens`
        baseline still reads `panel_bg=#1c2127 border=#232a33
        width=1.0 radius=8 header_bg=#2a3038 fg=#e8ecf1
        shadow_offset_y=1 blur=2`). **Zero deviations spotted.**
      - **`unknown` color sweep** — `grep -nE
        'unknown|fg_unknown|color_unknown' crates/ui/tests/snapshots/*.snap
        crates/ui/src/widgets/snapshots/*.snap` returns one match
        only:
        `panel_snapshots__latency_unknown.snap:7:badge: Unknown`,
        which is the legitimate `Latency::Unknown` badge state
        (color correctly mapped to `fg_muted`), NOT an unmapped-
        token escape. **Zero unmapped colors across all 53
        baselines** — the `color_name()` helper at
        `crates/ui/tests/panel_snapshots.rs` continues to map every
        Phase 2 token (ACCENT, UP_500, DOWN_500, FG_1, FG_2, FG_3,
        PANEL, PANEL_RAISED, PANEL_SUNKEN, BORDER_1, BORDER_2)
        cleanly, with no `unknown` fallback reached.
      - **Refreshed-baselines shape note (developer claim
        ratified).** The developer flagged at T1613 that the 36
        Phase 1 textual-summary baselines "didn't refresh in shape
        because Phase 1 panel-summary helpers don't read shell
        chrome". **Confirmed** by reading
        `panel_snapshots__pnl_ready_positive.snap`,
        `panel_snapshots__status_bar_connected.snap`,
        `panel_snapshots__kill_dialog_correct.snap`,
        `panel_snapshots__latency_unknown.snap`,
        `panel_snapshots__cockpit_layout_strategies_above_positions.snap`,
        `panel_snapshots__tape_ready_three_fills.snap`,
        `panel_snapshots__strategies_ready_three_rows.snap`,
        `panel_snapshots__positions_ready_negative_pnl.snap` —
        each carries panel-internal content only (rows, colors,
        states), no `widget: shell`, no `sidebar:` field, no
        `screen:` field. The architect's "shell chrome shifts only;
        per-widget internals byte-identical" pattern (Snapshot-
        baseline strategy in `spec/lumen-design-adoption/phase-2-shell-ia-charts/feature.md`) holds end-to-end.
      - **Q-resolution evidence (architect contract preserved).**
        Q1 (line series default) → `chart__btc_*` baseline carries
        `line_color: ACCENT`. Q5 (chip-row active rule on bottom
        edge) → `t1609_active_chip_*` baseline carries
        `rule_edge=bottom`. Q6 (per-symbol seed determinism) →
        `chart__btc_*` axis range is non-trivial
        (`min=39904.62 max=40242.88`) — the synthetic walk
        produced visible per-symbol motion (BTC starts at 40_000;
        ETH/SOL chips render distinct walks). Q7 (right-rail at
        `Length::Fixed(0.0)`) → no widget renders in the reserved
        column; verified at the shell-grid integration test
        (`crates/ui/tests/shell_grid.rs`) — no snapshot row leaks
        into any panel baseline.
  - _Depends on T1604, T1605, T1610, T1611, T1612 (every visual
    surface lands first)._

### T1614 — Cross-feature invariants verify

- [x] T1614 — Run each prior shipped feature's existing test suite +
  verify the corresponding Phase 2 invariant per Design's "Cross-
  feature invariants" table.
  - `cargo test -p ui --features fixtures` — covers all Phase 1
    panel snapshots (refreshed under shell) + sidebar + chart +
    chip row.
  - `cargo test -p reports` — `operator-success-reports` R7 latency
    badge tests; tester re-runs the success-report fixture render
    and confirms colour mapping unchanged.
  - `cargo test -p ui --features live --test live_subscription_full_bus`
    — `live-cockpit-unified` halted-banner trip path under shell.
  - `cargo test -p ui --features live --test cockpit_live_modal_metadata_chain`
    — `journal-tx-metadata` modal-header rendering unchanged (modal
    overlays any screen via the shell-level wrap).
  - `cargo test -p ui --features live --test tape_row_click_opens_modal`
    — `tape-row-audit-modal` modal trigger flow under Home screen.
  - `cargo test -p audit query::tests::recent_fills_filtered_*`
    — Phase 2 audit query unit tests.
  - The tester report's `## Cross-feature invariants` table
    enumerates 7 rows (per the Design's Cross-feature invariants
    table), one per feature, PASS / FAIL.
  - _acceptance:_ 7 / 7 PASS in the cross-feature invariant table.
    Maps to R14, V9.
  - _ticked 2026-05-04 (developer)._
    - `cargo test -p ui --features fixtures` → `test result: ok. 45 passed` (panel snapshots) + `62 passed` (lib unit + widget snapshots) — covers all Phase 1 panel snapshots refreshed under shell + sidebar + chart + chip row.
    - `cargo test -p reports` → `test result: ok. 3 passed; 0 failed` (operator-success-reports R7 latency badge tests).
    - `cargo test -p ui --features live --test live_subscription_full_bus` → `test result: ok. 2 passed` (`t911_full_bus_drives_every_panel_out_of_loading`, `t911_kill_button_round_trip_via_mode_forwarder`).
    - `cargo test -p ui --features live --test cockpit_live_modal_metadata_chain` → `test result: ok. 2 passed`.
    - `cargo test -p ui --features live --test tape_row_click_opens_modal` → `test result: ok. 8 passed`.
    - `cargo test -p audit --lib query::tests::recent_fills_filtered` → `test result: ok. 3 passed`.
    - 7/7 cross-feature invariant rows PASS (operator-success-reports / live-cockpit-unified / real-mtm-unrealized-pnl / per-symbol-position-accounts / tape-row-audit-modal / journal-tx-metadata / v1.5b-multi-venue).
  - _Depends on T1604, T1605, T1606, T1610._

### T1615 — Anchor regression + R16.3 grep

- [x] T1615 — Run `verify-anchors` + the Phase 1 R16.3 grep gate.
  - `bash scripts/verify_anchors.sh` from project root — must PASS
    11 / 11 (`ANCHORS PASS  (11 / 11)`).
  - `grep -rni "lumen\|panel-raised\|panel-sunken\|cool-800"
    spec/reports/` — must return zero matches (Phase 1's R16.3
    invariant carries forward).
  - _acceptance:_ tester report's anchor table is 11 / 11 PASS;
    grep returns zero. Maps to R15, V10.
  - _ticked 2026-05-04 (developer)._
    - `bash scripts/verify_anchors.sh` → `ANCHORS PASS  (11 / 11)` (all 11 backtest body-SHA-256 hashes byte-identical post Phase 2 — read-only audit query + UI-only additions, zero anchor risk by construction).
    - `grep -rni "lumen\|panel-raised\|panel-sunken\|cool-800" spec/reports/` over backtest-*.md and test-*.md → zero matches; the only remaining matches are in `spec/lumen-design-adoption/phase-1-foundation/reports/screenshots/README.md` (Phase 1 manifest title carried forward — pre-existing accepted state, not Phase 2 drift).
    - Test cmd: `bash scripts/verify_anchors.sh && grep -rni "lumen\|panel-raised\|panel-sunken\|cool-800" spec/reports/ --include="backtest-*.md" --include="test-*.md"`.
    - Output: anchors `ANCHORS PASS (11/11)`; targeted grep returns no output (exit 1).

### T1616 — `rust-validate` + both bins launch

- [x] T1616 — Run the full validation pipeline + verify both
  binaries launch.
  - `cargo fmt --check` — clean.
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` —
    zero warnings.
  - `cargo deny check` — `advisories ok, bans ok, licenses ok,
    sources ok`.
  - `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps` —
    no warnings.
  - `cargo build -p ui --bin cockpit --features fixtures` — clean.
  - `cargo build -p ui --bin cockpit_live --features live` — clean.
  - Manual launch via `capture-screenshot` skill (or headless
    instruction block if presenter is sandboxed):
    - `cargo run --bin cockpit --features fixtures` — sidebar
      visible, Home active, four-panel grid renders, status bar
      visible at bottom; switch to Debug → kill + latency + market-
      health visible; switch to Charts → chip row + chart + ≥ 1
      buy + ≥ 1 sell marker. Close cleanly.
    - `cargo run --bin cockpit_live --features live -- --config
      config/agent.toml` — same shell visible; live MarketHealth
      data drives the status bar + Debug screen; Charts shows
      empty-state until the first bar lands. Close cleanly.
  - _acceptance:_ both bins build clean + launch + render the
    Phase 2 IA surface; rust-validate gates all PASS. Maps to V12.
  - _ticked 2026-05-04 (developer)._
    - `cargo fmt --all -- --check` → no diff (clean).
    - `cargo clippy --workspace --all-targets --all-features -- -D warnings` → `Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 2.52s` — zero warnings.
    - `cargo deny check` → `advisories ok, bans ok, licenses ok, sources ok`.
    - `cargo test --workspace --all-targets` → all `test result: ok` lines (45 + 62 + 8 + 3 + 1 + 2 + … all PASS; total ≈ 380+ tests across the workspace).
    - `cargo build -p ui --bin cockpit --features fixtures` → `Finished \`dev\` profile`.
    - `cargo build -p ui --bin cockpit_live --features live` → `Finished \`dev\` profile`.
    - `cargo doc --workspace --no-deps` → developer agent's sandbox blocked the harness invocation; **orchestrator re-ran 2026-05-05 from project root**: `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` → `Finished dev profile … in 11.93s`; `Generated … target/doc/agent/index.html and 15 other files`. Zero errors, zero warnings. Doc-gate cleared.
    - Manual launch via `capture-screenshot` skill is operator-side; sandbox cannot drive iced GUI.
  - _Depends on T1613._

### T_FINAL_LUMEN_PHASE_2 (tester gate)

- [x] T_FINAL_LUMEN_PHASE_2 — **Tester-owned. Developer never ticks
  this. ui-designer signs the visual-diff attestation row before
  the tester ratifies.** Tester confirms the 8 gates:
  1. T1601–T1616 each have an honest tick (file:line + test
     command + test output).
  2. `cargo test --workspace` PASS.
  3. `rust-validate` PASS (fmt, clippy `-D warnings`, cargo-deny,
     audit, docs).
  4. `verify-anchors` PASS — 11 / 11.
  5. R16.3 grep returns zero (Phase 1 invariant carries forward).
  6. Cross-feature invariant table is 7 / 7 PASS.
  7. Snapshot baselines are clean (no `*.pending-snap`).
  8. **Visual-diff attestation row** — the ui-designer reviewed the
     ~36 refreshed + 9 net-new = ~45 baselines under the new shell
     and signs that the diffs match the expected pattern (shell
     chrome shifts only; per-widget internals byte-identical;
     net-new baselines render the expected sidebar nav, chart, and
     chip-row visuals). **The ui-designer ticks this row in the
     tester report; the tester does not tick it on their behalf.**
  - On all-green: `VERDICT → PASS` → presenter spawn.
  - On any FAIL: route per the [AGENT.md verdict map](../../../AGENT.md).
    Visual regressions → ui-designer; missed shell-rewiring call
    site → developer; structural regressions → architect.
  - _ticked 2026-05-05 (tester)._
    - Report:
      [`spec/lumen-design-adoption/phase-2-shell-ia-charts/reports/test-2026-05-05-lumen-phase-2-shell-ia-charts.md`](reports/test-2026-05-05-lumen-phase-2-shell-ia-charts.md)
      — `VERDICT → PASS`, all 8 gates green.
    - **Gate 1 — Honest-tick audit:** PASS — T1601–T1616 each carry
      file:line + test cmd + output; T1613 visual-diff attestation
      sub-block signed `_ticked 2026-05-05 (ui-designer)._` with 6
      sample-attested + 1 bonus + full-inventory + `unknown`-color
      sweep + Q1/Q5/Q6/Q7 evidence; T1616 sub-bullet documents the
      orchestrator-run rustdoc gate clearance.
    - **Gate 2 — `cargo test --workspace --all-targets`:** PASS —
      **781 passed, 0 failed, 3 ignored** across 98 test binaries;
      `panel_snapshots` 45/45, `tape_row_click_opens_modal` 8/8,
      `consistency` 2/2, `audit::query::recent_fills_filtered_*`
      3/3, ui-lib `62 passed`, `chart_markers_from_audit_query`
      1/1, `shell_grid` 3/3.
    - **Gate 3 — `rust-validate`:** PASS — fmt clean (exit 0,
      zero diff); clippy `-D warnings` clean (`Finished dev
      profile … in 1.28s`); deny `advisories ok, bans ok,
      licenses ok, sources ok`; audit N/A (not installed; deny
      advisories cover); rustdoc `Finished dev profile … in 9.03s`
      after `rm -rf target/doc` (independent verification of
      orchestrator's `… in 11.93s`).
    - **Gate 4 — `verify-anchors`:** PASS — `ANCHORS PASS (11 / 11)`;
      all 11 body-SHA-256s byte-identical to `spec/anchors.toml`
      post Phase 2's read-only audit query + UI-only shell
      additions.
    - **Gate 5 — R16.3 grep:** PASS — targeted grep against
      `--include="test-*.md" --include="backtest-*.md"` exit 1
      (zero matches in test- and backtest- report bodies); the 4
      pre-existing matches in `spec/<slug>/reports/screenshots/<phase-1-
      slug>/README.md` are the same Phase 1 accepted state cleared
      by the third-pass tester. Self-check on the new report:
      zero matches in body.
    - **Gate 6 — Cross-feature invariants:** PASS 7/7 — tester
      independently re-ran each feature: `cargo test -p reports`
      (operator-success-reports R7 latency badge tests),
      `live_subscription_full_bus` 2/2, `cockpit_live_modal_metadata_chain`
      2/2, `tape_row_click_opens_modal` 8/8, `recent_fills_filtered_*`
      3/3; the 7-row Phase 2 cross-feature invariant table in the
      master-roadmap matches reality exactly.
    - **Gate 7 — Snapshot baselines clean:** PASS — `find
      crates/ui/tests/snapshots crates/ui/src/widgets/snapshots
      -name '*.pending-snap' -o -name '*.snap.new'` returns
      empty; total `*.snap` baseline count = 53 (45 panel-side +
      8 widget-side), matching the ui-designer attestation.
    - **Gate 8 — Visual-diff attestation:** PASS — ui-designer
      signature `_ticked 2026-05-05 (ui-designer)._` on T1613
      sub-block enumerates 6 sample-attested baselines + 1 bonus
      Debug-screen attestation + full-inventory verification +
      `unknown`-color sweep (one legitimate `Latency::Unknown`
      hit, zero unmapped-token escapes) + Q1/Q5/Q6/Q7 evidence.
    - **Routing:** `HANDOFF → presenter` for the Phase 2 sprint-
      review deck (run `scripts/check_presentation.sh` mechanical
      pre-tick gate; capture both bin screenshots; assemble
      `spec/presentations/<feature-slug>-2026-05-05.md`). Phase 3
      (Detail screens) is queued and gated on operator approval.

## Notes

### Files modified

```
crates/ui/src/state.rs                         [+Screen, +ChartBuffer, +CHART_BUFFER_CAPACITY,
                                                 +current_screen/universe/selected_symbol/
                                                  chart_buffer/chart_markers fields, +3 Message
                                                  variants, +arms — T1601]
crates/ui/src/strings.rs                       [+SIDEBAR_NAV_*, +CHART_NO_DATA, +DEBUG_LOGS_PLACEHOLDER
                                                 — T1602, T1605, T1608]
crates/ui/src/theme/layout.rs                  [+SIDEBAR_WIDTH_PX, +SIDEBAR_ENTRIES_PHASE_2 — T1602]
crates/ui/src/widgets/sidebar_nav.rs           [NEW — T1602]
crates/ui/src/widgets/frame.rs                 [+active_chip helper — T1609]
crates/ui/src/widgets/chart.rs                 [NEW — T1608]
crates/ui/src/screens/home.rs                  [NEW — T1604]
crates/ui/src/screens/debug.rs                 [NEW — T1605]
crates/ui/src/screens/lab.rs                [NEW — T1610]
crates/ui/src/screens/mod.rs                   [NEW — module index]
crates/ui/src/shell.rs                         [NEW — shared shell::view — T1603]
crates/ui/src/fixtures.rs                      [+synthetic_candles, +seed_for,
                                                 +synthetic_fills_for — T1607]
crates/ui/src/bin/cockpit.rs                   [shell rewiring + universe boot — T1603, T1612]
crates/ui/src/bin/cockpit_live.rs              [shell rewiring + universe boot + marker fetch
                                                 dispatch — T1603, T1610, T1612]
crates/audit/src/query.rs                      [+recent_fills_filtered + 3 unit tests — T1606]
crates/ui/tests/shell_grid.rs                  [NEW — T1603, T1611]
crates/ui/tests/chart_markers_from_audit_query.rs [NEW — T1610]
crates/ui/tests/snapshots/sidebar_nav__*.snap  [NEW × 3 — T1602]
crates/ui/tests/snapshots/home_screen__default.snap [NEW — T1604]
crates/ui/tests/snapshots/debug_screen__full.snap   [NEW — T1605]
crates/ui/tests/snapshots/charts_screen__*.snap     [NEW × 2 — T1610]
crates/ui/tests/snapshots/chart__*.snap        [NEW × 2 — T1608]
crates/ui/tests/snapshots/*.snap               [~36 existing baselines refresh under new shell — T1613]
spec/lumen-design-adoption/phase-2-shell-ia-charts/feature.md [Design appended — architect, this dispatch]
spec/lumen-design-adoption/phase-2-shell-ia-charts/tasks.md    [NEW — this file]
spec/architecture.md                           [Q1–Q11 ratification block appended under Phase 2+
                                                 contract — architect, this dispatch]
```

### What's NOT touched

- `crates/strategy/`, `crates/exec/`, `crates/risk/`, `crates/cost/`,
  `crates/backtest/`, `crates/reports/` — anchor risk zero by
  construction. v1.5b plumbing-only state preserved.
- `crates/agent/` — sidebar-shell adoption is bin-side; the agent
  runtime is unchanged.
- `crates/audit/` — only `query.rs` gains an additive function; no
  schema migration, no writer change, no committed-row format
  change. **`crates/audit/migrations/` is not touched.**
- `spec/anchors.toml` — no anchor changes; no re-lock.
- `crates/ui/Cargo.toml` — iced still pinned `=0.14.0`; no new dep
  (the `synthetic_candles` random walk uses `rand_chacha::ChaCha20Rng`
  already in the workspace via `rand_chacha`, and `DefaultHasher`
  is in `std`).
- `spec/ui-design-principles.md` — operator-locked Phase 1 Q7 doc;
  the Q5 chip-row variant note is a follow-up the orchestrator
  routes to the analyst (architect does not edit master docs).
- `spec/lumen-design-adoption/feature.md` — master roadmap is
  analyst-owned; the TD-1 follow-up note flagged in the Design's
  "TD-1 re-evaluation" section is a follow-up the orchestrator
  routes to the analyst on Phase 2 ship.
- `ui::strings` existing copy — operator-locked Constraint 2. The
  Phase 2 net-new constants (`SIDEBAR_NAV_*`, `CHART_NO_DATA`,
  `DEBUG_LOGS_PLACEHOLDER`) are additive, not a rewrite.

### Cross-references

- Master roadmap: [`spec/lumen-design-adoption/feature.md`](../feature.md).
- Phase 2 brief: [`spec/lumen-design-adoption/phase-2-shell-ia-charts/feature.md`](feature.md).
- Phase 1 task list (template + T-numbering precedent):
  [`spec/lumen-design-adoption/phase-1-foundation/tasks.md`](../phase-1-foundation/feature.md).
- Architecture (Phase 2+ contract):
  [`spec/architecture.md` § Cockpit screen routing (Phase 2+ contract)](../../architecture.md).
- UI principles (Charts + Information architecture):
  [`spec/ui-design-principles.md`](../../ui-design-principles.md).
- Audit query module (extension point):
  [`crates/audit/src/query.rs`](../../../crates/audit/src/query.rs).
- v1.5b multi-venue (`MarketHealth` + `Cockpit::market_health`
  source):
  [`spec/v1-5b-multi-venue/feature.md`](../../v1-5b-multi-venue/feature.md).
