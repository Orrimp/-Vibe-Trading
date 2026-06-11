---
title: Paper-trade-live infrastructure audit — gap inventory for Route C planning
date: 2026-05-29
authors: [ui-designer]
status: survey
tags: [survey, route-c, paper-trade-live, continuity, infra, audit, dev-note]
related:
  - spec/product.md
  - spec/dev-notes/post-v3-strategy-direction-2026-05-29.md
  - spec/architecture/08-recovery-and-backups.md
  - spec/architecture/adr/0041-trader-crate-split.md
  - spec/architecture/adr/0044-activity-aggregator-pattern.md
  - spec/backlog.md
---

# Paper-trade-live infrastructure audit — gap inventory for Route C planning

> **This is a SURVEY / dev-note, NOT a feature brief.** No `[[req]]` rows,
> no Queue → Active promotion, no feature folder, no code commitment.
> ui-designer enumerates what would need to land to honor the v3 success
> criterion ("90 days continuous paper-trading on real Binance with simulated
> fills, weekly auto-generated operator success reports, lesson-card memory
> demonstrably accumulating, uptime > 99%, zero risk-limit breaches"). If
> the operator picks Route C next week, the gap list converts directly
> into the next analyst pass's brief queue.

## § What "paper-trade-live continuity" means

Concrete checklist (12 sub-criteria the operator's "is it running?" question silently decomposes into):

1. Agent process runs 24/7 against real Binance WS.
2. WS reconnects with backoff + audit-journaled `feed_reconnect`.
3. SMA / v1-momentum emits Signal → Decision → Order → simulated Fill into the ledger without operator interaction.
4. Audit ledger reconciles every bar (start_cash + Σ(fills) + Σ(P&L) = end_equity).
5. Boot-id uptime intervals open/close cleanly across every restart (T806).
6. Kill-switch trip flatten works from operator GUI button AND from `.halt` file.
7. Process survives Mac reboot (launchd / login-item auto-restart with last good config + ledger).
8. Position state survives crash (on restart, agent rehydrates open positions from ledger; no double-trade).
9. Stale-data watchdog publishes `MarketHealth::{Fresh,Stale,Recovered}`; cockpit surfaces visible banner.
10. Reflection memory: every closed trade writes a `LessonCard`; trader retrieves top-K cards before the next decision.
11. Weekly operator success reports auto-generate (Monday 00:00 UTC) without operator intervention.
12. Out-of-band alerting when laptop is closed / operator away — WS disconnect, persistent stale-data, kill trip, reconciliation fail, drawdown approach, LLM budget 80%.

## § Current state (what is BUILT — audit-mode)

- **`cockpit_live` binary EXISTS** (`crates/ui/src/bin/cockpit_live.rs`). Unified agent + iced GUI in one process; iced owns main thread, agent runtime owns side-thread tokio. Kill button → `KillSwitch::trip` via `rt_handle.spawn`; T809 audit dual-write live.
- **Per-venue ingest** (T1408+T1409): Binance always-on, Coinbase + Kraken opt-in. Panic-isolated supervisors per venue. Stale-data watchdog publishes `MarketHealth` 1 Hz; status-bar tile consumes it.
- **Audit ledger writes during live + backtest** — `Ledger::open_with_tick_bus` (ADR-0044) tees writes onto broadcast bus for trail-mirror + reflection-audit-tick consumers.
- **Reflection-memory writer task** is plumbed in `crates/agent/src/main.rs` behind `cfg.reflection.enable_writer` (default `false`). `exec::PaperEnginePublisher::on_trade_close` enqueues `LessonCardWriteRequest` on flat-close.
- **Trader crate split** (ADR-0041): `crates/trader/llm_forecaster/` owns the LLM consumer; `crates/strategy/` is reflection-free per gate test.
- **Activity tape producer trio** (ADR-0042/0044): LLM + Training + Audit producers fan into the activity bus → cockpit status strip.
- **LLM-forecaster integration** shipped v0.1.0-PARTIAL (Phase F Assistant lights up; operator sees reasoning + retrieved lesson cards live). Determinism + sample-size questions open.
- **Lumen design phases 1–5 shipped.** Sidebar shell + Home/Charts/Strategies/Risk/Audit/Debug screens + status-bar anchor + cockpit-toast-queue v0.1.0 / v0.2.0 cleanup. Phase 6 Assistant slot reserved, gated on v2 LLM.
- **Cockpit-Backtest (Lab) end-to-end** shipped (lab-yahoo-realdata v0.1.2).
- **In-process cron scheduler exists** (`crates/agent/src/cron.rs`) but is gated by `--features in_process_cron` — NOT in default build.
- **WS reconnect** in `crates/data/src/binance.rs` — exponential backoff (1s base, 60s cap), audit-journaled.

## § Gap inventory (what is MISSING for 24/7 continuity)

**G1 — Process supervision (Mac reboot, crash auto-restart).** No launchd plist, no systemd unit, no "set as login item" runbook. After Mac reboot or panic, `cockpit_live` stays down until manual reopen. Sub-criterion (7) fails. Subtlety: `cockpit_live` opens an iced GUI window, so launchd respawn is operator-hostile. Right shape is headless `trading` bin under launchd + on-demand `cockpit_live` GUI for monitoring — current framing assumes a single process serves both roles.

**G2 — Position state recovery on cold-boot.** `audit::query::open_positions_at(ts)` (line 1731) EXISTS but neither `cockpit_live::main` nor `agent::runtime::run` calls it at startup. On crash + restart the engine starts flat even if the ledger says BTCUSDT +0.4 is open. Silent double-trading risk on first signal. Sub-criterion (8) fails.

**G3 — Reflection writer disabled by default.** `cfg.reflection.enable_writer = false` in `config/agent.toml`. The post-trade lesson-card loop is plumbed end-to-end but the producer is dormant. Sub-criterion (10) silently fails; the v3 success metric "lesson-card memory demonstrably accumulating" cannot be observed.

**G4 — Periodic distillation / post-mortem regeneration is offline-only.** Per `crates/reflection/src/lib.rs:23` ("periodic distillation deferred"), cards land on disk but are not clustered into operator-reviewable rules during live runs. v0 distillation runs at backtest M-FINAL, not paper-trade-live. The operator's "what is the agent learning?" loop never closes during continuous operation.

**G5 — Operator success reports cron is feature-flagged off in default build.** `crates/agent/src/cron.rs` is `#[cfg(feature = "in_process_cron")]`; `crates/agent/Cargo.toml:20` keeps it off by default. No operator-facing "Enable weekly reports" toggle in the cockpit either. Sub-criterion (11) silently fails on a fresh checkout.

**G6 — No out-of-band operator alerting.** When `cockpit_live` runs and the laptop lid closes, no channel wakes the operator on WS disconnect > N min, persistent stale-data, kill trip, reconciliation fail, drawdown approach, LLM budget 80%. Toasts are in-process only. Sub-criterion (12) fails. **Operator-decide load-bearing**: transport choice (Apple Push, ntfy.sh, Pushover, SMTP relay, Slack webhook) — different setup + cost + privacy tradeoffs.

**G7 — Cockpit IA gap: no "is it running" landing tile.** Home renders positions + tape + P&L assuming agent has been running. There is no first-class "uptime N days, last reconnect M h ago, last LessonCard W min ago, last weekly report K d ago, next report due J h" tile. Operator's opening question ("is it running?") has no single-glance answer.

**G8 — Cockpit-Backtest is local-trigger-only.** Lab runs on-demand. No scheduled "weekly OOS backtest sanity check against live equity divergence" surface. For 90-day paper continuity the operator wants a small regression watchdog ("live Sharpe diverges from rolling backtest Sharpe by > X bp → flag"). Currently a manual workflow.

**G9 — Ledger backup runbook documented but not automated.** `08-recovery-and-backups.md` specifies nightly `sqlite3 .backup` + weekly Parquet rsync. The implementation hook is described as "a tokio task in the agent binary" — that task does not appear in `agent::runtime::run`. A clean run survives; a disk corruption rolls back to the most recent **manual** snapshot.

**G10 — No persistent operator notes / journal surface.** When the operator spots "weird behavior on 2026-06-12" in the cockpit, there is no place to leave a note that survives restart and surfaces in the next weekly report's "Open risks". Cockpit is read-only; the audit ledger is double-entry-only.

**G11 — Lumen Phase 6 Assistant slot reserved but inert.** Right-rail column-track is `Length::Fixed(0.0)` until v2 LLM ships. For Route C the operator gains differentiator visibility if Assistant wakes against the shipped C5 v0.1.0-PARTIAL output. Sub-criterion (10) is mediated by this surface.

**G12 — Reconciliation failure has no cockpit banner.** `crates/agent/src/reconciler.rs` exists; a reconciliation failure logs `warn!` and continues. No visible cockpit banner, no audit-row marker on the affected transaction, no operator recipe.

## § Per-gap rework-cost estimate

| Gap  | Days | Crates                                | M-T1 needed?                    | Operator-decide Q                                                  |
|------|------|---------------------------------------|---------------------------------|--------------------------------------------------------------------|
| G1   | 3–5  | runbooks + ops/plist + agent          | YES — headless vs GUI split     | headless `trading` + on-demand cockpit, OR unified process?         |
| G2   | 2–3  | agent + audit (caller) + ui banner    | YES (small) — startup ordering  | warn-restore or block-boot on position-reconcile fail?              |
| G3   | 0.5  | config + docs                         | NO — config flip                | flip default `true` always, or only when `mode = paper`?            |
| G4   | 8–14 | reflection + agent (cron) + ui Memory | YES — distillation contract     | clustering algorithm + storage shape                                |
| G5   | 1–2  | Cargo.toml + config + runbooks        | NO — feature flag flip          | always-on vs config gate; output dir naming                         |
| G6   | 5–10 | NEW `crates/notify` + agent + ui + config | YES — transport choice ADR    | **Q-critical** transport (Apple Push / ntfy.sh / Pushover / SMTP / Slack) + which events fire |
| G7   | 3–5  | ui (Home restructure) + agent uptime helper | YES — IA review              | replace Home or add new "Status" screen above Home?                 |
| G8   | 5–8  | agent (cron) + backtest + reports + ui divergence panel | YES — divergence contract | rolling window length; alert threshold bp; per-strategy or portfolio |
| G9   | 2–3  | agent (cron task) + runbooks          | NO — additive task              | backup destination path; retention window (30d? 90d?); checksum?    |
| G10  | 3–4  | NEW `crates/journal` + ui Notes + ledger join | YES — small             | operator-only notes table vs append-to-audit-trail?                 |
| G11  | 4–6  | ui Assistant + trader llm_forecaster wakeup | YES — re-eval v0.2.0 standing-Q | wake Assistant against C5 PARTIAL vs wait for v0.2.0?            |
| G12  | 2–3  | agent (reconciler) + ui banner        | NO                              | severity ladder (warn-only / flatten / halt)                        |

Totals: ~38–66 dev-days. **3 of 12 are M0-only** (G3, G5, G9 — pure config / feature-flag / additive task). The other 9 need at least one architect M-T1.

## § Prioritized 3-month roadmap (assuming Route C)

**Month 1 — close the silent-failure cluster** (cheapest wins, highest operator confidence per dev-day):
- **W1**: G3 + G5 + G9 (reflection writer default-on, cron default-on, automated ledger backup). All config / feature-flag / additive. ui-designer + developer pair. ~4 dev-days. Sub-criteria 10, 11, DR-restore close.
- **W2–W3**: G7 (Home "is it running" tile). UI-designer lead. Uptime + last-reconnect + last-LessonCard + last-report + next-report-due in one surface. ~3–5 dev-days.
- **W3–W4**: G2 (position recovery on cold-boot). Architect M-T1 on startup-ordering + warn-vs-block decision. Closes silent double-trade risk. ~2–3 dev-days.

**Month 2 — close the operator-presence-required cluster:**
- **W5–W7**: G6 (out-of-band alerting). Single largest gap; transport decision load-bearing. After landing, laptop-closed + away-from-keyboard is safe. ~5–10 dev-days.
- **W7–W8**: G1 (process supervision via launchd). Pair with G6 so alerting catches relaunch events. ~3–5 dev-days.

**Month 3 — close the moat-perception cluster + polish:**
- **W9–W11**: G4 (live periodic distillation). Product-differentiator payoff per product.md § Differentiator. Memory loop visible to operator. ~8–14 dev-days (largest new code).
- **W11–W12**: G8 + G10 + G11 + G12 (divergence watchdog, operator notes, Assistant slot wake, reconciler banner). Polish wave; pick 2–3 highest-leverage. ~5–10 dev-days.

**Continuous (every week):** `operator-cockpit-smoke` per the cockpit-smoke operator-manual capture pattern. Cheap (~30 min/wk operator); catches regressions and keeps the 99% uptime target honest.

## § Headline + Month-1 recommended priority pair

**Roadmap headline:** "Route C — Month 1 closes the silent-failure cluster (G3+G5+G9, ~4 dev-days) and makes the cockpit answer 'is it running' (G7+G2, W2–W4); Month 2 unlocks laptop-closed operation (G6+G1); Month 3 surfaces the moat (G4+G11)."

**Month-1 analyst-recommended priority pair:**
1. **G3 (reflection writer default-on)** — single-line config flip. Immediately unlocks the v3 success-metric "lesson-card memory demonstrably accumulating" observable signal. Without it, Month 3's G4 distillation has nothing to cluster.
2. **G7 ("is it running" Home tile)** — operator confidence + single-glance answer to the question this whole effort exists to serve. Compounds the product-differentiator perception per the durable-contract logic of the 2026-05-29 Route C post-mortem.

## § What does NOT need work (validates the investment story)

- **Audit ledger writes during live + backtest.** Tick-bus tee (ADR-0044) is robust; chart-of-accounts bootstrap is idempotent; double-entry reconciles by construction.
- **WS reconnect with exponential backoff.** Binance feed has audit-journaled `feed_reconnect`; multi-venue panic isolation (T1408) + stale-data watchdog (T1409) cover transient drops.
- **Kill-switch trip path.** Operator GUI button + `.halt` file watcher both work; T809 dual-write into ledger verified by integration test.
- **Trader crate split** (ADR-0041) — `strategy` is reflection-free per gate test; reflection consumers live in `trader`.
- **Activity tape producer trio** (ADR-0042/0044) — LLM + Training + Audit fan into one bus; cockpit status strip consumes deterministically.
- **Lumen design system phases 1–5.** Theme tokens, sidebar shell, all six trading screens, status bar, toast queue — shipped + operator-approved.
- **Cockpit-Backtest (Lab) end-to-end** (lab-yahoo-realdata v0.1.2).
- **Determinism** of backtest replay path (R15 anchor invariant). Live additions surgically gated to never touch backtest bytes.
- **Cost budget plumbing.** `CostBudget` + `LedgerCostSink` wired at agent boot.

## Cross-references

- Product framing — [`spec/product.md`](../product.md)
- Post-v3 Route C source — [`post-v3-strategy-direction-2026-05-29.md`](post-v3-strategy-direction-2026-05-29.md)
- DR + backups policy — [`08-recovery-and-backups.md`](../architecture/08-recovery-and-backups.md)
- Trader crate split — [`ADR-0041`](../architecture/adr/0041-trader-crate-split.md)
- Activity aggregator — [`ADR-0044`](../architecture/adr/0044-activity-aggregator-pattern.md)
- Lumen Phase 6 stub — [`phase-6-assistant-slot/feature.md`](../lumen-design-adoption/phase-6-assistant-slot/feature.md)
- Backlog UI Queue — [`spec/backlog.md § UI / cockpit`](../backlog.md)
