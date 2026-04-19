# v0-paper-sma — reference summary

Single-file reference replacing the 16 `.txt` logical-state artifacts + 16
`.png` mockups that used to live in this directory. Optimized for future AI
sessions to validate, extend, and spec-drive new work without walking the
full source tree.

## 1. Feature status

| Field | Value |
|---|---|
| Feature brief | [../../features/v0-paper-sma.md](../../features/v0-paper-sma.md) |
| Task list | [../../tasks/v0-paper-sma.md](../../tasks/v0-paper-sma.md) |
| Tasks complete | 35 / 35 (T01 → T_FINAL_B) |
| Final verdict | **PASS** — [test-2026-04-19-0615-v0-paper-sma-ship.md](../test-2026-04-19-0615-v0-paper-sma-ship.md) |
| Test count | 124 passing, 0 failing, 3 `#[ignore]` (live Binance WS) |
| Doc tests | 0 errors (rename `core` → `trading_core` fixed the stdlib shadow) |
| Determinism gate | body-SHA256 byte-identical across two runs at seed `0xC0FFEE` |
| Prometheus | `/metrics` returns all 12 R9.2 names |
| Locked on | 2026-04-19 |

## 2. What v0 shipped

12 Rust crates, all workspace-level `clippy -D warnings` clean:

| Crate | Role |
|---|---|
| `trading_core` | Domain primitives — Decimal `Money<C>`, gated `Order::new`, `Signal`, `Decision`, `Bar`, `Tick`, `Fill`, `Position`, typed error enums. `trybuild` compile-fail tests enforce invariants. |
| `data` | Market-data ingestion — `MarketDataSource` trait with `BinanceFeed` (WS), `ReplayFeed` (Parquet), `FakeFeed`; `trade_aggregation` tick→bar; clock-skew detector with Prometheus gauge. |
| `features` | `features::sma` adapter over `kand` (batch) + `quantedge-ta` (streaming), cross-checked to `Decimal::new(1, 8)`. |
| `strategy` | `Strategy` plug-in trait + `StrategyRegistry` (compiled-in for v0) + `sma_crossover` reference impl. |
| `risk` | `size_and_validate` fixed-fraction sizer with per-symbol exposure cap; returns `Result<Order, RiskError>` (never panics). |
| `backtest` | `MatchingEngine` trait + `PaperEngine` (bps slippage + taker fee + bar-VWAP, seeded `ChaCha20Rng`); `backtest` binary writes markdown reports. |
| `audit` | `sqlx-ledger` on SQLite, 13-account chart, `post_fill` double-entry, `audit::query` read-only surface. Minute-boundary reconciler runs as tokio task. |
| `cost` | `CostEvent` / `CostSink` / `LedgerCostSink` / `CostBudget` scaffold; zero emitters in v0 (wired for v0.5 LLM calls without schema change). |
| `exec` | Paper-only `ExecRouter` wiring the matching engine into the agent pipeline. |
| `models` | Stub — populated in v2.5+. |
| `llm` | Stub — populated in v0.5 (first `sentiment_analyst`). |
| `agent` | Top-level binary `trading`: construct all subsystems, kill switch (halt file + heartbeat), Prometheus exporter on `:9100`, broadcast-bus API (`Arc<EventBus>`) for cockpit. |
| `ui` | iced 0.14 cockpit binary. Single design source of truth via `ui::theme` + `ui::strings`; `tests/consistency.rs` enforces zero inline strings / hex in widgets. v0.5 extends with the `strategies` panel (right column, above Open positions — Q4 resolution) plus three new broadcast subscribers (`strategy_loaded` / `strategy_swapped` / `strategy_error`). |

## 3. Canonical backtest runs (v0 ship artifacts)

Seed `0xC0FFEE`, fixed-fraction 0.1 sizing, 2 bps slippage, 0.04% taker, $100 000 initial. Both deterministic.

| Scenario | Report | Body SHA-256 | Final equity | Wall-clock |
|---|---|---|---|---|
| `btc-2023-1m-sma-cross` | [backtest-20260419-060409](../backtest-20260419-060409-btc-2023-1m-sma-cross.md) | `fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c` | $47 290.03 | 0.2 s |
| `btc-2024-h1-sma-cross` | [backtest-20260419-060410](../backtest-20260419-060410-btc-2024-h1-sma-cross.md) | `345ee0c0d485a44b8b4adabcf5e2af36e82224034e1f8bc8d66694378352a574` | $67 241.80 | 0.1 s |

Both scenarios show losses — **expected** per the analyst's hypothesis in the feature brief ("SMA cross on 1m is a known underperformer; we test the harness, not the edge"). A positive Sharpe would have been a red flag that fees/slippage were mis-modelled. Ledger imbalance on both = 0.

## 4. Cockpit panel state reference

Single design contract: all copy flows from `crates/ui/src/strings.rs`, all colors/spacing from `crates/ui/src/theme.rs`. Consistency audits (`crates/ui/tests/consistency.rs`) fail the build if any widget inlines a string literal or hex color. Five panels × four states = 20 rendered combinations (T528 added the `strategies` panel in v0.5).

Copy is the exact operator-facing text. String keys reference the `strings.rs` constant that carries it; theme tokens reference `theme.rs`.

### 4.1 `tape` — live fills feed (up to 200 rows)

| State | Copy | Key visual |
|---|---|---|
| loading | "Connecting to the fill stream…" | `color::FG_MUTED` body |
| empty | "No fills yet. Waiting for the first bar from BTCUSDT." | `color::FG_MUTED` body |
| error | "Can't read the fill stream: Trading agent disconnected. Check the agent log and restart it." — `TAPE_ERROR_PREFIX` + `CONNECTION_CHANNEL_CLOSED` | `color::NEG` prefix |
| ready | Rows: `Time  Symbol  Side  Price  Qty  Fee` (most recent first, ≤ 200). Buy → `color::POS`, Sell → `color::NEG`; other columns neutral. Right-aligned monospace. Pause toggle label: "Pause" (idle) → "Resume" (paused). | |

### 4.2 `positions` — open positions

| State | Copy | Key visual |
|---|---|---|
| loading | "Loading positions from the ledger…" | `FG_MUTED` |
| empty | "No open positions. Strategy is armed and watching." | `FG_MUTED` |
| error | "Ledger error while reading positions: Trading agent disconnected. Check the agent log and restart it." — `POS_ERROR_PREFIX` + `CONNECTION_CHANNEL_CLOSED` | `NEG` prefix |
| ready | Columns: `Symbol  Qty  Cost  Mark  P&L  P&L %  Exposure %`. Zero-qty rows filtered out (T17). P&L sign colors via `theme::color_for_delta` (`POS` / `NEG` / `FG_MUTED`). Exposure % shown to two decimals. | |

### 4.3 `pnl` — P&L card

| State | Copy | Key visual |
|---|---|---|
| loading | "Reading equity from the ledger…" | `FG_MUTED` |
| empty | "No equity recorded yet. First reconciliation pending." | `FG_MUTED` |
| error | "Ledger error while reading equity: Trading agent disconnected. Check the agent log and restart it." — `PNL_ERROR_PREFIX` + `CONNECTION_CHANNEL_CLOSED` | `NEG` prefix |
| ready | Equity (big, `text::DISPLAY` 22 px) + rows: `daily_return`, `cash`, `unrealized`, `realized`. Sign color only on deltas — `daily_return`/`unrealized`/`realized` via `color_for_delta`; `cash`/`equity` always `FG`. Right-aligned monospace. | |

### 4.4 `kill` — stop-trading control

| State | Copy | Key visual |
|---|---|---|
| loading *(== idle, no async state)* | Button "Stop trading"; help "Cancels open orders, flattens every position, and halts the agent. Requires a typed confirmation." | Big `NEG` button, `FG` label, `FG_MUTED` help |
| empty *(== loading)* | Same as loading — no halt, no dialog | same |
| error *(== halted, sticky)* | Banner "AGENT HALTED" (`text::DISPLAY` in `NEG`); hint "Remove .halt and re-arm from the operator runbook before resuming."; runbook link "Open kill-switch runbook" → `spec/runbooks/kill-switch.md` (via `KILL_RUNBOOK_LINK_PATH`). | `NEG` banner, `WARN` hint, `ACCENT` link |
| ready *(== confirming)* | Dialog title "Confirm stop trading"; body "This cancels every open order, sells each open position at market, and puts the agent into a halted state. Type the phrase below to confirm."; label "Type HALT BTC to confirm"; phrase typed `"HALT BTC"`, matched `true`, Confirm enabled; mismatch shows `KILL_PHRASE_MISMATCH_HINT` in `color::WARN`. | Disabled confirm until phrase matches |

### 4.5 `strategies` — loaded strategies + swap log

v0.5 addition (tasks T522–T528). Placement: **right column, above Open positions** per the Q4 resolution in [architecture.md § v0.5 cockpit strategies panel layout](../../architecture.md#v05--cockpit-strategies-panel-layout-q4--confirmed-2026-04-19). Columns: `Strategy`, `Hash`, `Status`, `Last event`, `Signals / 60s`, `Holds position`. Footer: last ten `StrategyEventView`s colored by event kind (Load → `ACCENT`, Swap → `WARN`, Unload → `FG_MUTED`, Reject → `NEG`).

| State | Copy | Key visual |
|---|---|---|
| loading | "Loading active strategies…" — `STRATEGIES_LOADING` | `FG_MUTED` body |
| empty | "No strategies loaded. Drop a TOML under config/strategies/ to begin." — `STRATEGIES_EMPTY` | `FG_MUTED` body, the `config/strategies/` path is carried verbatim in the copy so the operator knows exactly where to add a TOML |
| error | "Can't read strategies: Trading agent disconnected. Check the agent log and restart it." — `STRATEGIES_ERROR_PREFIX` + `CONNECTION_CHANNEL_CLOSED` | `NEG` prefix via the shared `error_body` frame helper |
| ready | Rows: `Strategy  Hash  Status  Last event  Signals / 60s  Holds position`. Status pill colors: Ready → `POS`, Loading → `FG_MUTED`, Error → `NEG`. Error-marked rows render a caption-sized `NEG` badge beneath them carrying the `error_summary` (R8 "malformed TOML, old strategy keeps running" visual). Last-event column uses plain verbs: "loaded" / "swapped" / "unloaded" / "rejected". `Holds position` uses "yes" / "no". Numbers right-aligned monospaced. Recent-events footer (strategies-recent-event rows): `loaded` → `ACCENT`, `swapped` → `WARN`, `unloaded` → `FG_MUTED`, `rejected` → `NEG`. | |

String keys on this panel (all prefixed `STRATEGIES_*`): `PANEL_STRATEGIES_TITLE`, `STRATEGIES_LOADING`, `STRATEGIES_EMPTY`, `STRATEGIES_ERROR_PREFIX`, `STRATEGIES_COL_*` (6), `STRATEGIES_STATUS_*` (3), `STRATEGIES_EVENT_*` (4), `STRATEGIES_POSITION_HELD` / `STRATEGIES_POSITION_FLAT`. Theme tokens: reused only — `color::POS`, `color::NEG`, `color::WARN`, `color::ACCENT`, `color::FG`, `color::FG_MUTED`; zero new tokens (deliberate per the three-goal contract).

Subscriptions driving this panel (via `ui::live`, feature `live`): `strategy_loaded` → `Message::StrategyLoaded`, `strategy_swapped` → `Message::StrategySwapped`, `strategy_error` → `Message::StrategyLoadError`. `RecvError::Lagged(n)` → log-and-continue; `RecvError::Closed` → `Message::StrategiesError(STRATEGIES_ERROR_PREFIX + CONNECTION_CHANNEL_CLOSED)`.

## 5. Latency badge thresholds

Source of truth: `ui::theme::latency` in `crates/ui/src/theme.rs`.

| Range | Band | Token |
|---|---|---|
| `< 500 ms` | OK | `color::POS` |
| `< 2 s` | WARN | `color::WARN` |
| `< 10 s` | HIGH | `color::NEG` |
| `≥ 10 s` | HALTED | `color::NEG` + banner + auto-kill |

## 6. Kill-switch safety phrase

Typed-confirm phrase: **`HALT BTC`** (from `strings::KILL_SAFETY_PHRASE`). iced renders Confirm disabled (`on_press = None`) unless typed input exactly equals the phrase. See R7 and [kill-switch runbook](../../runbooks/kill-switch.md).

## 7. For future AI sessions — validation hints

- **Single source of truth** for panel-state semantics: this file + `crates/ui/src/state.rs` (iced Model + Message enum).
- **Consistency is test-enforced.** If `cargo test -p ui` passes, the UI is token-faithful by construction — no inline strings (`no_inline_user_visible_strings_in_widgets`) and no inline hex (`no_inline_hex_colors_in_widgets_or_state`). A new widget must add its copy to `ui::strings` and its colors via `ui::theme` or the consistency tests fail.
- **Adding a new panel state:** (a) extend `PanelState<T>` (in `crates/ui/src/state.rs`), (b) add copy to `ui::strings`, (c) add a row to the table above, (d) add an `insta` snapshot in `crates/ui/tests/panel_snapshots.rs`.
- **Regenerating visual PNG mockups** (optional — not needed for validation, only for PR review aesthetics): `scripts/render_panel_mockups.py` rendered the originals using the tokens above. The script's current input is `.txt` artifacts that were compacted into this README; to regenerate it, the script would need a refactor to read from this table or to reconstitute `.txt` stubs. Low priority — real screenshots come from `cargo run --bin cockpit --features fixtures` on an actual display (see [smoke checklist](../ui-week2-smoke-checklist-2026-04-18.md)).
- **Verifying a v0 regression:** `cargo test --workspace && cargo run --release --bin backtest -- --scenario btc-2023-1m-sma-cross --seed 0xC0FFEE`, compare body-SHA256 to `fc2e3b4a…` above. Byte-identical match = PASS.
- **Building on top in v0.5+:** the broadcast-bus API contract the cockpit subscribes to lives in [dev-week2-broadcast-api-2026-04-18.md](../dev-week2-broadcast-api-2026-04-18.md). New v0.5 runtime events (`StrategyLoaded`, `StrategySwapped`, `StrategyLoadError`) extend this bus; corresponding `Message` variants + `ui::live` subscription handlers + the `strategies` panel row above all landed together (T522–T528). Future extensions follow the same playbook: add types to `trading_core`, broadcast channel to `agent::EventBus`, `Message` variant + `update` arm in `ui::state`, widget in `ui::widgets`, a snapshot, and a row in this reference.

## 8. Manual smoke + screenshot capture

The operator-facing smoke checklist (walks through loading / empty / error / ready for each panel, plus both kill-switch triggers) lives at [ui-week2-smoke-checklist-2026-04-18.md](../ui-week2-smoke-checklist-2026-04-18.md). That file also carries the deferred-manual PNG capture instructions; `cargo run --bin cockpit --features fixtures` on a display + OS screenshot tool produces the real iced-rendered PNGs.
