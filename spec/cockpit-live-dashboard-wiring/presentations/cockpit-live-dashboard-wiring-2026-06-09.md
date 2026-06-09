---
slug: cockpit-live-dashboard-wiring
mode: release
status: draft
owner: presenter
audience: human-operator
updated: 2026-06-09
generated: 2026-06-09T08:40:00Z
---

# Cockpit Live dashboard wiring — the equity curve + KPI strip now show the live agent

> This deck closes **cockpit-live-dashboard-wiring** (v0.1.0, tester VERDICT → PASS +
> cockpit-smoke PASS, HEAD `e5301d2`). It is the natural follow-on to last week's
> Baseline panel: that feature proved the cockpit's `equity_curve` / `kpi_strip`
> widgets render a real curve + KPI strip **from a file**; this one points the
> **same two widgets** at the **running paper agent's live feed** instead of a
> permanent "Loading" placeholder. Small, self-contained, ~100% UI wiring — the
> data was already arriving at the UI every bar; the panels just dropped it on the
> floor. Every number below traces to a snapshot file the tester locked, a headless
> test I ran live this session, or the orchestrator's smoke log — and (capability
> boundary, see § Visual / hands-on evidence) I did **not** boot the windowed
> cockpit; the render evidence is the textual panel-snapshots plus an operator
> hands-on recipe that is the real payoff here.

## TL;DR

The cockpit's **Live screen now monitors** — the equity curve and the 6-card KPI
strip render the paper agent's real state and update every bar, instead of showing
a permanent "Loading" placeholder, all by wiring two stubbed panels to a
session-scoped equity buffer with **zero new crate edge, zero new widget, zero new
theme token**.

## What changed

- **The two stubbed panels on the Live screen are now live.** `screens/live.rs:58`
  and `:66` were hard-wired to `&PanelState::Loading` (the module header literally
  annotated them "no live feed yet — Phase F"). They now read model-backed state
  (`&model.live_equity_curve` and `&model.live_kpi`). The two render widgets
  (`equity_curve`, `kpi_strip`) are reused **verbatim** — same signature as the
  Baseline screen.
- **A session-scoped equity buffer feeds the curve + live KPIs.** Each per-bar
  `PnlRefreshed` snapshot the UI already receives now also appends one
  `(timestamp, total_equity)` point to a bounded ring buffer
  (`LIVE_EQUITY_BUFFER_CAP = 2_880` = 48 h of 1-min bars; ~140 KB worst case), and
  the curve + KPI strip are re-derived on each append. **No new bus channel, no new
  message, no agent/exec change** — it rides the existing `pnl` cadence.
- **Two KPI cards are live and honest; four show the honest absent state.**
  **Max DD** and **Total return (session-to-date)** carry real live numbers derived
  from the accumulated series. **Sharpe / CAGR / Win rate** render `—` (there is no
  live Sharpe/CAGR/win-rate math in `core` — same finding the Baseline panel hit;
  *not fabricated*). **Trades** shows `0` (no live session fill-counter exists yet —
  a named follow-on, not a bug). A static **"Session to date"** caption sits under
  the strip so the Total-return card can never be mistaken for an annualized result.

## Why

The Live screen is the operator's window onto the running paper-trading agent. Its
recent-activity, positions, P&L, latency, and market-health panels were already
live-wired — but its two **headline** panels, the equity curve and the KPI strip,
were hard-coded to `Loading` and never updated. The data they needed was already
flowing: the reconciler publishes a `PnlSnapshot` every bar
(`reconciler → publish_pnl`), the UI subscribes and stores it
(`stream_pnl → Message::PnlRefreshed → model.pnl`) — the panels simply never
rendered it as a curve / strip. This was the ui-designer's queued build-out after
`cockpit-baseline-panel`, and it reuses that feature's render path and the
`cockpit_live` plumbing wholesale: the only genuinely new logic is a small UI-side
ring buffer that accumulates the per-bar points into the `EquitySeries` the widget
already knows how to draw. (Source: [`feature.md`](../feature.md) "Why" + § Design.)

## What you can do now

| Action | Command |
|--------|---------|
| **Run the unified agent+UI and watch the panels populate live** (the real payoff — full recipe in § Visual / hands-on evidence) | `cargo run -p ui --release --bin cockpit_live --features live` |
| See the fixtures-only cockpit (stays Loading — no agent, by design) | `cargo run -p ui --bin cockpit --features fixtures` |
| Re-run the 7 live wiring tests I ran this session | `cargo test -p ui --lib -- pnl_refresh_sequence_populates_live_equity_curve live_kpi_strip_loading_at_one_point_ready_at_two live_equity_buffer_drops_out_of_order_and_allows_equal_ts live_equity_buffer_is_bounded_ring live_panels_reset_on_fresh_cockpit live_kpi_strip_max_drawdown_is_live pnl_error_drives_live_panels_to_error_no_panic` |
| Re-run the headless smoke (paints the Live route, no panic) | `cargo test -p ui --test headless_emulator_smoke` |
| Read the design (the equity buffer, the 1-point trap, the KPI mapping) | open [`feature.md`](../feature.md) § Design |

## Live demo

I cannot boot the windowed cockpit from a sub-agent (capability boundary — see
§ Visual / hands-on evidence), so the live ground-truth I ran this session is the
**headless render + wiring suite**: the deterministic equivalent of "the panels
feed and paint". These ran at HEAD `e5301d2`.

```
$ cargo test -p ui --lib -- pnl_refresh_sequence_populates_live_equity_curve \
    live_kpi_strip_loading_at_one_point_ready_at_two \
    live_equity_buffer_drops_out_of_order_and_allows_equal_ts \
    live_equity_buffer_is_bounded_ring live_panels_reset_on_fresh_cockpit \
    live_kpi_strip_max_drawdown_is_live pnl_error_drives_live_panels_to_error_no_panic

running 7 tests
test state::tests::live_kpi_strip_max_drawdown_is_live ... ok
test state::tests::live_equity_buffer_drops_out_of_order_and_allows_equal_ts ... ok
test state::tests::live_kpi_strip_loading_at_one_point_ready_at_two ... ok
test state::tests::pnl_error_drives_live_panels_to_error_no_panic ... ok
test state::tests::live_panels_reset_on_fresh_cockpit ... ok
test state::tests::pnl_refresh_sequence_populates_live_equity_curve ... ok
test state::tests::live_equity_buffer_is_bounded_ring ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 428 filtered out; finished in 0.62s

$ cargo test -p ui --test headless_emulator_smoke
running 3 tests
test headless_emulator_paints_live_route ... ok
test headless_emulator_boots_cockpit_and_renders ... ok
test headless_emulator_paints_baseline_route ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.67s
```

These seven tests **are** the feature working: `pnl_refresh_sequence_populates_live_equity_curve`
is the feed→curve proof (buffer grows point-by-point; curve goes Loading→Ready at
≥1 point); `live_kpi_strip_loading_at_one_point_ready_at_two` is the 1-point trap
(the strip stays Loading at one point and only goes Ready at two — see § Verification
note); `live_kpi_strip_max_drawdown_is_live` proves Max-DD is a real derived number
(1000→1200→900 yields Max DD 0.25, session return −0.10); and
`pnl_error_drives_live_panels_to_error_no_panic` proves a channel error degrades both
panels to a muted Error body rather than crashing. `headless_emulator_paints_live_route`
is the one that matters most for "does it survive in the real shell": it boots the
fixtures cockpit on the default **Live** route with no agent, asserts both panels are
Loading and the first frame is non-empty, and does not panic.

## Visual / hands-on evidence

**Capability note — why there is no PNG screenshot in this deck.** Per the AGENT.md
sub-agent capability boundary, booting the live windowed cockpit
(`cargo run --bin cockpit_live` with a window) is **orchestrator-only**; I cannot
open a window or grab a real screenshot, and faking one is forbidden. So the
rendered-structure evidence here is the **committed textual panel-snapshots** the
tester locked (these capture exactly what the two panels compose, in both themes),
and below them is a **hands-on recipe** so you can boot the unified agent+UI and
watch the panels populate live yourself. **This recipe is the real payoff of the
feature** — the headless tests prove the wiring; the recipe is how you see it move.

### Committed snapshot evidence (the rendered structure, both themes)

Three `.snap` files pin the byte-stable rendered structure of the Live screen, locked
by `panel_snapshots::live_screen` (all 3 GREEN this session, part of the 99-test
panel-snapshots suite):

| Snapshot file | What it pins |
|---------------|--------------|
| [`…__steady_state.snap`](../../../crates/ui/tests/snapshots/panel_snapshots__live_screen__live_snapshot__steady_state.snap) | the **Loading default** (no feed — the fixtures / fresh-boot state) |
| [`…__ready_dark.snap`](../../../crates/ui/tests/snapshots/panel_snapshots__live_screen__live_snapshot__ready_dark.snap) | **Ready** state, dark theme (seeded ≥2 points) |
| [`…__ready_light.snap`](../../../crates/ui/tests/snapshots/panel_snapshots__live_screen__live_snapshot__ready_light.snap) | **Ready** state, light theme (seeded ≥2 points) |

The **Loading default** (`steady_state`) renders verbatim as (the actual committed
`.snap` body — this is what a fresh boot / the fixtures `cockpit` shows):

```
screen: Live
theme: dark
headline: Live
system_health_label: System health
equity_curve: No equity data placeholder
kpi_strip: Backtest metrics unavailable
session_caption: Session to date
llm_spend_label: LLM spend
llm_spend_tile: —
bottom_left: positions (state=loading)
bottom_right: agent_feed (state=loading)
```

The **Ready** state (`ready_dark`, after a ≥2-point seed) renders verbatim as:

```
screen: Live
theme: dark
headline: Live
system_health_label: System health
equity_curve: ready points=2
kpi_strip: ready
  card Total return: 0.10%
  card CAGR: —
  card Sharpe: —
  card Max DD: 0.00%
  card Win rate: —
  card Trades: 0
session_caption: Session to date
llm_spend_label: LLM spend
llm_spend_tile: —
bottom_left: positions (state=loading)
bottom_right: agent_feed (state=loading)
```

Two things to notice for honesty's sake:

1. **The four honest cards.** Sharpe / CAGR / Win rate render `—` (no live math in
   `core`); Trades renders `0` (no live fill-counter yet). These are the honest
   absent states — **not fabricated numbers** — exactly matching the Lab strip's
   existing `—` Sharpe and the Baseline panel's same `core`-has-no-Sharpe finding.
   The `ready_dark` / `ready_light` snapshots are the byte-stable proof of this.
2. **The "Session to date" caption is binding honesty.** The Total-return card shows
   a **session-to-date** return (first accumulated point = session open), not an
   annualized or multi-year figure. The static caption makes that unambiguous and
   prevents the live session from being read as "the baseline result". The `bottom_*`
   rows still show `state=loading` because the snapshot harness seeds only the equity
   buffer — positions/agent-feed are the already-live panels this feature didn't
   touch.

### Hands-on recipe — boot the unified agent+UI and watch the panels populate

Self-contained operator verification. This is the **unified** `cockpit_live` binary
(agent + iced in one process sharing one `EventBus`), so the paper agent actually
trades the passive baseline and feeds the panels live — unlike the fixtures `cockpit`
which has no agent and stays Loading.

- **Command:**
  ```bash
  cargo run -p ui --release --bin cockpit_live --features live
  ```
  (The `--features live` flag is required — it pulls in the `agent` and builds the
  unified binary. `--release` because the first launch builds release; see Timing.)
- **Steps:**
  1. Wait for the release build, then the cockpit window to open. It **boots to the
     Live screen by design** (`cockpit_live` sets `Screen::Live` as the default
     route), so no navigation is needed — but if you've clicked away, select **Live**
     in the sidebar to return.
  2. On first paint (before the agent's first bar closes) the equity-curve panel
     shows its **"No equity data"** skeleton and the KPI strip shows the
     **"unavailable"** six-dash body. **This is correct, not a bug** — the panels are
     Loading until the agent feeds them.
  3. Wait for the agent to trade. As each bar closes (~once per minute at the paper
     config's bar cadence), watch the **equity curve grow point-by-point** and the
     KPI strip's **Max DD** and **Total return** cards **populate and update**. The
     strip flips from the six-dash "unavailable" body to live cards once **two**
     points have accumulated (the 1-point trap — see § Verification).
  4. Confirm the honest absent cards stay `—`: **Sharpe / CAGR / Win rate** never
     show a number, and **Trades** stays `0`. The **"Session to date"** caption sits
     under the strip.
  5. (Optional) Toggle the theme → the whole screen re-renders correctly in the
     other theme (the dark+light Ready snapshots prove this is wired).
- **Timing:** **first launch builds release (~3–5 min)** on a warm target, longer
  cold. Once the window is up it paints instantly; you then wait roughly **one bar
  interval (~1 min)** for the first point and **two intervals (~2 min)** for the KPI
  strip to flip to live cards. Leave it running a few minutes to watch the curve grow.
- **Expected result:** the equity curve grows a live line and the KPI strip shows a
  live **Max DD** + **Total return (session)** that update each bar, with
  Sharpe/CAGR/Win-rate `—` and Trades `0`; both themes render correctly; no blank
  panel, no crash. The "Session to date" caption is present.
- **Failure diagnosis:**
  - *Window opens but the curve/strip never leave Loading after several minutes* →
    the agent isn't feeding `PnlRefreshed`. Check the terminal for agent errors
    (reconciler not advancing bars / no market data). The panels Loading **with no
    agent** is the correct fixtures behavior; under `cockpit_live --features live`
    you should see the agent's bar-close log lines and the panels should populate.
  - *Target won't compile, "requires the features: live"* → you dropped
    `--features live`; re-add it. (The plain `cockpit` bin is fixtures-only and stays
    Loading — that's the wrong binary for watching live data.)
  - *The strip shows six dashes even though the curve has a point* → that's the
    **expected** 1-point state; the strip intentionally waits for the **second**
    point before showing cards (the `is_all_absent` trap, § Verification). One more
    bar and it flips to live cards.
  - *KPI strip shows live cards but Sharpe/CAGR/Win-rate are `—` and Trades is `0`* →
    **expected and honest** — no live Sharpe/CAGR/win-rate math exists in `core` and
    no live fill-counter is wired yet (both named follow-ons). Not a defect.
- **Cleanup:** close the cockpit window (Cmd-Q); the agent stops with the process.
  The equity buffer is **session-scoped and not serialized** — it lives only in
  memory and is gone on close, so nothing is written to disk and the next boot starts
  with an empty curve (the correct live-monitor session-open state).

## Verification

The feature's acceptance criteria are AC1–AC7 (the feature.md `## Acceptance
criteria` section serves as the verification matrix). Each is mapped to its passing
test from the tester report
([`test-2026-06-09-cockpit-live-dashboard-wiring.md`](../reports/test-2026-06-09-cockpit-live-dashboard-wiring.md)
§4), and I re-ran the 7 core wiring tests + the headless smoke live this session.

| AC | Description | Status | Evidence |
|----|-------------|--------|----------|
| AC1 | Live panels render the agent feed; curve grows, KPIs update as snapshots arrive (Loading→Ready) | VERIFIED | `pnl_refresh_sequence_populates_live_equity_curve` — buffer grows per `PnlRefreshed`; curve Loading at 0, Ready at ≥1 (ran live, PASS) |
| AC2 | Four panel states behave (Loading / Ready / Empty / Error) for both panels | VERIFIED | `live_kpi_strip_loading_at_one_point_ready_at_two` (Loading+Ready), `pnl_error_drives_live_panels_to_error_no_panic` (Error, no panic), `live_panels_reset_on_fresh_cockpit` (Loading default) — all PASS live; Empty maps to Error on channel-close (non-blank, ratified by tester Open Q (b)) |
| AC3 | Fixtures `cockpit` smoke — first-frame Live route, no agent, panels Loading, no panic | VERIFIED | `headless_emulator_paints_live_route` (empty buffer + both Loading + non-empty first frame, no panic — ran live); the orchestrator cockpit-smoke gate (see § Numbers, cited log: 0 panics) |
| AC4 | Lumen-consistent — consistency/contrast/layout green; no hardcoded colors/strings; both themes | VERIFIED | `consistency` / `contrast` / `layout_invariants` green (in the 99-test panel-snapshots suite); `LIVE_SESSION_RETURN_CAPTION` in `strings.rs`; zero new clippy warnings from feature files (tester §2) |
| AC5 | Honest live labels — "session" caption; absent cards `—`, never a fake number | VERIFIED | `LIVE_SESSION_RETURN_CAPTION = "Session to date"`; the `live_snapshot__ready_dark`/`ready_light` snapshots assert Sharpe/CAGR/Win `—` (read verbatim above); no fabricated number |
| AC6 | Panel snapshots (Loading + Ready, both themes) | VERIFIED | `live_snapshot__steady_state` (Loading, regenerated), `live_snapshot__ready_dark`, `live_snapshot__ready_light` (seeded ≥2 pts) — all 3 GREEN (in the 99-test suite) |
| AC7 | No new crate edge, no new widget, no new theme token | VERIFIED | `crates/ui/Cargo.toml` not in the implementation commit's changed files (tester §2); `LIVE_EQUITY_BUFFER_CAP` is a retention const, not a visual token; no new widget; `live.rs` reuses `equity_curve` + `kpi_strip` verbatim |

**The 1-point trap (the load-bearing correctness detail).** At the very first
accumulated point, the live `BacktestMetrics` is `{ total_return: 0, max_dd: 0,
trades: 0, all present-flags false }` — which is **byte-identical** to the
`kpi_strip::is_all_absent` "unavailable" sentinel. Without a guard the strip would
wrongly render six dashes instead of real "Total return 0.00% / Max DD 0.00%" cards.
The resolution: the **strip stays Loading until the buffer holds ≥2 points** (one
real session delta), while the **curve renders from ≥1 point** (a single dot is a
valid 1-point curve). This intentional 1-point-curve / 2-point-strip split is
asserted by `live_kpi_strip_loading_at_one_point_ready_at_two` (PASS) and is why,
in the hands-on recipe, you'll see the curve appear one bar before the strip flips
to live cards.

## Numbers that matter

- **Tests:** UI suite **435** lib unit tests GREEN (0 fail) + all integration suites
  GREEN (682+ total excl. the whitelisted pre-existing failure). The 7 core wiring
  tests + 1 fixtures-smoke (`headless_emulator_paints_live_route`) + 3 Live panel
  snapshots are the feature's tests — all PASS (I re-ran the 7 wiring tests and the
  smoke live this session). (Tester report §3.)
- **The one non-green test in the suite is pre-existing and does not gate PASS:**
  `lab_run_engine::inner::h3_in_memory_equals_cached_disk` — the network-dependent
  `--features live` backtest test, confirmed by the tester to fail identically on
  clean HEAD (same as the `cockpit-baseline-panel` precedent). Unrelated to this
  feature.
- **Anchors:** **119 / 119 PASS** (`verify-anchors`, ran live this session). No
  anchored file was touched — this is a read-only live-monitor UI wiring; the trace
  row's `anchors` column is correctly `[]` (N/A).
- **Visual gate:** **51 / 51 PASS**. Exactly **3** baselines regenerated — the
  `live__recent_activity_with_chevron` PNG triple (floor / typical / operator) — and
  the diff is **only** the new "Session to date" caption + the wired panels (still
  Loading in the feedless fixture). The other 48 visual baselines are untouched.
  (Tester report §7.)
- **cockpit-smoke gate (orchestrator pre-tick):** **PASS — 0 panics**. (Log:
  [`cockpit-smoke-2026-06-09T06-32Z.log`](../reports/cockpit-smoke-2026-06-09T06-32Z.log)
  — the only line is the pre-existing deprecated-`Screen::Home` warning, then
  `Finished` + `Running target/debug/cockpit`; no panic, clean exit.)
- **spec-lint:** **94 violations in 2 categories** at HEAD `e5301d2` (87 dead-link +
  7 trace-broken-path), exit 0 — **exactly the documented `audit-2026-06-08` baseline
  (94/2-cat). No regression.** (I re-ran it live; see § Open decisions note 2 for why
  the tester's mid-pass report showed transient 95/4.)
- **New crate edges / widgets / theme tokens:** **0 / 0 / 0** (AC7).
- **The buffer, in one line:** `LIVE_EQUITY_BUFFER_CAP = 2_880` = 48 h of 1-min bars,
  ~140 KB worst case — a bounded ring, session-scoped, not serialized, empty each
  `cockpit_live` boot.

## Open decisions

_No decision is required to ship — this is a clean release-mode PASS. Three points
are surfaced below so they're visible, not buried; none blocks approval. The first
two are honest scope boundaries the operator should be aware of (each has a named
follow-on already); the third is a spec-lint reconciliation note._

1. **The equity curve is session-scoped — it resets each `cockpit_live` boot (by
   design; durable history is a named follow-on).** No equity *history* exists
   anywhere on the live/exec/agent side (the reconciler keeps only a scalar
   `last_equity` it overwrites each tick), so the UI accumulates the per-bar points
   itself into an in-memory, not-serialized buffer. **This is the correct behavior
   for a live monitor** — a fresh process starts at the session open and grows as
   the agent trades — but it means **closing and reopening the cockpit loses the
   curve**. A durable agent/exec-side equity history (survives restart, reusable by
   other surfaces) is a larger exec-side change, deferred to a named follow-on
   **`live-equity-history-durable`** (unscheduled). *No action needed now* — just be
   aware the curve is session-to-date, which is exactly what "run the paper agent via
   `cockpit_live`" should show.

2. **Trades shows `0` and Sharpe/CAGR/Win-rate show `—` — honest absences, each with
   a deferred follow-on (no fake numbers).** There is no live session fill-counter on
   the model (`FillReceived` only pushes the capped `tape` window — `tape.len()` is
   not a session total), so Trades renders `0`; and there is no live Sharpe/CAGR/win-
   rate math in `core` (same finding the Baseline panel hit), so those three cards
   render `—`. A true session fill-counter is ~10 lines + a test (deferred to the same
   follow-on); live Sharpe needs a return-series + annualization methodology decision
   + new `core` math (a separate, larger item). **Both are deliberately out of scope
   for this monitor-wiring feature** — flagged so you're not surprised when those four
   cards stay quiet in the hands-on recipe. *If you want the live Trades counter now,
   say so in the notes and it routes back as a small add.*

3. **spec-lint reconciliation — the tester's mid-pass report shows 95/4, HEAD shows
   the 94/2 baseline (no regression).** During the tester pass the ui-designer's files
   transiently carried a non-enum `status: ui-done` (2 `missing-frontmatter`) plus a
   +1 dead-link, which the tester self-corrected to `status: tester-done` in-pass. I
   re-ran spec-lint at HEAD `e5301d2` this session and it reports **94 in 2 categories**
   (87 dead-link + 7 trace-broken-path, exit 0) — **matching the `audit-2026-06-08`
   baseline exactly**, no missing-frontmatter, no new dead-link, no orphan. **Nothing
   for you to do** — I'm noting it only so the tester-report-vs-deck count discrepancy
   is explained, not a surprise.

## Approval

- [ ] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes / feedback

_empty until operator fills_

## Feedback log

_empty — no rejections routed_

## Changelog

- 2026-06-09 (presenter): initial release-mode deck for cockpit-live-dashboard-wiring
  v0.1.0 (tester VERDICT → PASS + cockpit-smoke PASS, HEAD `e5301d2`). Live evidence
  run this session at HEAD `e5301d2`: the 7 core wiring tests 7/7 GREEN, headless-
  emulator smoke 3/3 GREEN, anchors 119/119 PASS, spec-lint 94/2-cat (baseline, no
  regression). Rendered-structure evidence is the 3 committed Live panel-snapshots
  (steady_state Loading + ready dark/light, read verbatim into the deck — sub-agent
  cannot boot the windowed cockpit) + a 6-section hands-on recipe to run the unified
  `cockpit_live --features live` and watch the panels populate (the real payoff).
  Surfaced the two honest scope boundaries (session-scoped curve; Trades `0` /
  Sharpe·CAGR·Win `—`, each with a named follow-on) + the spec-lint reconciliation.
  Approval block ships UN-ticked.
