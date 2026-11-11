//! F6/F9 agent→ui "last-mile" RELAY tests (advisor-forward-plan ADR-0062 §D4 /
//! advisor-llm-narration ADR-0064 §D3).
//!
//! ## Why this file exists — the bug class the fixture render tests CANNOT catch
//!
//! The F6/F9 fixture render tests (`forward_plan_populated_render.rs`,
//! `leaderboard_narration_render.rs`) construct the cockpit state DIRECTLY and
//! render it. They prove the populated state *draws* — but they BYPASS the
//! channel, so they cannot catch the actual bug that shipped: the cockpit recipe
//! that consumes the agent→ui channel was never wired, so `plan_rx` was never
//! drained (F6 Plan screen stayed Empty) and the narration-outcome receiver was
//! never drained (F9 "Explain" could only ever FellBack).
//!
//! These tests drive the EXTRACTED relay functions the recipes wrap
//! (`forward_plan_stream_impl` / `narration_outcome_stream_impl` — the
//! `async_stream` bodies over an `rx`, the `lab_progress_recipe_stream.rs`
//! precedent) and assert the full channel→`Message`→`update`→populated-state
//! path end to end. They also pin the iced→agent "Explain" request BUILD
//! (`narration_request_from_mirror`) over two fixtures that differ in every
//! fact it carries. The ENQUEUE (the `try_send` in the binary's
//! `BakeoffNarrationRequested` dispatch, `crates/ui/src/bin/cockpit_live.rs`)
//! lives in a bin target an integration test cannot link, so it is NOT
//! exercised here — a residual, not a claim.
//!
//! Story 2-63 (lab-recipe-test-harness v0.3, Wave A S1) adds the full
//! `Recipe::stream` path for both recipes (`ForwardPlanRecipe`,
//! `NarrationOutcomeRecipe` had zero test references), multi-item relays that
//! a `while`→`if` mutation cannot survive, and pinned narration preconditions
//! (seeded so they can fail, not assumed from `Cockpit::new()` defaults).
//!
//! ## Non-vacuity contract (bug-log #66 / story 2-63 AC3)
//!
//! Every relay test proves it RAN: exact message counts and order, and the
//! stream end observed as `Ok(None)` inside a timeout that FAILS the test. No
//! timeout is ever accepted as "drained" — [`next_or_fail`] panics on elapse.
//! "Nothing more is buffered" is OBSERVED with [`assert_pending`], never read
//! off a counter the test increments itself. A state precondition is asserted
//! only where a seeded prior state lets it fail; checks that hold by
//! construction (a cell built as `Some(rx)` one line earlier, a
//! `Cockpit::new()` default) are not asserted at all.
//!
//! ## T-T4 falsification probe table (story 2-63 AC4)
//!
//! Each probe is ONE edit to production code, located by symbol + the quoted
//! line text (line numbers rot). Procedure:
//!
//! 1. snapshot the file (`cp -p <file> <snap>`) and apply the edit;
//! 2. run `cargo test -p ui --test forward_narration_relay` — the named
//!    test(s) go RED;
//! 3. restore from the snapshot, THEN `touch <file>`. A restore that keeps the
//!    snapshot's older mtime (`cp -p`) leaves cargo's fingerprint "fresh": the
//!    next run compiles nothing and re-executes the MUTANT binary;
//! 4. re-run. The GREEN counts only if that run's output shows
//!    `Compiling ui` and the file's SHA-256 equals the pre-probe one.
//!
//! Probe numbers are shared across the three Wave A files
//! (`forward_narration_relay.rs`, `bakeoff_progress_relay.rs`,
//! `sweep_progress_relay.rs`).
//!
//! | # | symbol (file) | quoted line today | mutation | expected RED |
//! |---|---|---|---|---|
//! | P1 | `ui::live::forward_plan_stream_impl` (`crates/ui/src/live.rs`) | `while let Some(plan) = rx.recv().await {` | `while` → `if` | `forward_plan_relay_yields_one_message_per_plan_in_order` (1 message, wanted 2) |
//! | P2 | `ui::live::forward_plan_stream_impl` | `while let Some(plan) = rx.recv().await {` | → `while let Some(plan) = None::<agent::ForwardPlan> {` | `forward_plan_relay_terminates_on_sender_drop` (0 messages, wanted 1), the in-order test, `forward_plan_relay_yields_message_and_populates_state`, and `forward_plan_recipe_stream_takes_receiver_once_and_relays` (its `assert_pending` finds the stream ENDED) |
//! | P14 | `ui::live::forward_plan_stream_impl` | `debug!(channel = "forward_plan", "plan channel closed — stopping recipe");` | append `futures::future::pending::<()>().await;` | `forward_plan_relay_yields_message_and_populates_state`, `forward_plan_relay_terminates_on_sender_drop`, the in-order test, the recipe test (each final `Ok(None)` times out → panic) |
//! | P3 | `<ForwardPlanRecipe as Recipe>::stream` | `forward_plan_stream_impl(rx_opt)` | → `forward_plan_stream_impl(None)` | `forward_plan_recipe_stream_takes_receiver_once_and_relays`, at its `assert_pending` right after `stream()` ("the stream already ENDED") — before any send |
//! | P15 | `<ForwardPlanRecipe as Recipe>::stream` | `forward_plan_stream_impl(rx_opt)` | insert `if rx_opt.is_none() { return Box::pin(futures::stream::pending()); }` before it | `forward_plan_recipe_stream_takes_receiver_once_and_relays`, at the SECOND recipe on the taken cell (its 1 s `Ok(None)` times out). The only test that drives `stream()`'s own empty-cell path; `forward_plan_relay_none_yields_nothing` bypasses `stream()` and stays green |
//! | P4 | `ui::live::narration_outcome_stream_impl` | `while let Some(outcome) = rx.recv().await {` | `while` → `if` | `narration_outcome_stream_resolves_two_bakeoff_runs_through_one_receiver`: after run 1 its `assert_pending` finds the stream ENDED, so run 2 can never be relayed |
//! | P4b | `ui::live::narration_outcome_stream_impl` | `debug!(channel = "narration_outcome", "narration channel closed — stopping recipe");` | append `futures::future::pending::<()>().await;` | the two-runs test, `narration_outcome_relay_ready_yields_message_and_sets_state`, `narration_outcome_relay_fellback_yields_message_and_sets_state`, the narration recipe test (each final `Ok(None)` times out → panic) |
//! | P16 | `ui::live::narration_outcome_stream_impl` | `yield Message::BakeoffNarrationCompleted(narration_outcome_to_ui(outcome));` | append a second `yield Message::BakeoffNarrationCompleted(crate::leaderboard::NarrationOutcome::FellBack);` | the two-runs test, at its post-run-1 `assert_pending` ("an unexpected extra message": exactly ONE message per run); the ready/fellback/recipe tests' final `Ok(None)` receives the extra message too |
//! | P5 | `<NarrationOutcomeRecipe as Recipe>::stream` | `narration_outcome_stream_impl(rx_opt)` | → `narration_outcome_stream_impl(None)` | `narration_outcome_recipe_stream_takes_receiver_once_and_relays`, at its `assert_pending` right after `stream()` |
//! | P17 | `<NarrationOutcomeRecipe as Recipe>::stream` | `narration_outcome_stream_impl(rx_opt)` | insert `if rx_opt.is_none() { return Box::pin(futures::stream::pending()); }` before it | `narration_outcome_recipe_stream_takes_receiver_once_and_relays`, at the SECOND recipe on the taken cell |
//! | P6 | `ui::state::update`, arm `Message::BakeoffNarrationRequested` (`crates/ui/src/state.rs`) | `st.begin_narration();` | delete the line | `narration_outcome_relay_ready_yields_message_and_sets_state`, `narration_outcome_relay_fellback_yields_message_and_sets_state`, the two-runs test (`request_explain`'s `InFlight` precondition) |
//! | P18 | `ui::leaderboard::LeaderboardScreenState::begin_run` (`crates/ui/src/leaderboard/state.rs`) | `self.narration = NarrationState::NotRequested;` | delete the line | the ready, fellback and two-runs tests, all at `cockpit_with_ready_bakeoff`'s `NotRequested` precondition (the seeded prior run's `Ready` narration survives the new run) |
//! | P19 | `ui::live::narration_request_from_mirror` | `winner_id: rec.winner.clone(),` | → `winner_id: candidate_ids[0].clone(),` | `narration_request_from_mirror_carries_the_on_screen_facts` (the `BenchmarkWins` mirror crowns its 2nd row). Its single-fixture predecessor stayed GREEN under this probe: the `ActiveWins` fixture's winner IS row 0 |
//!
//! Gated on `live` (the only build where the wiring + the `agent` types exist),
//! NOT on `target_os` (no pixels here — pure channel/state logic, deterministic
//! on every OS).

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::stream::BoxStream;
use futures::{FutureExt, StreamExt};
use iced::advanced::subscription::{EventStream, Recipe};
use rust_decimal_macros::dec;
use smol_str::SmolStr;
use tokio::time::timeout;

use ui::leaderboard::{BakeoffReportMirror, NarrationState, OutcomeKind};
use ui::live::{ForwardPlanRecipe, NarrationOutcomeRecipe};
use ui::state::{Cockpit, Message, PanelState, update};

// ── Builders ──────────────────────────────────────────────────────────────────

/// A populated, deterministic `agent::config::ForwardPlan` (the agent-side type
/// the supervisor sends over `plan_tx`). SMA-cross, LONG, BTCUSDT — mirrors the
/// `fake_forward_plan` fixture's shape but as the AGENT type, so the relay
/// genuinely exercises the `ForwardPlanView::from_plan` adapter at the boundary.
fn agent_forward_plan() -> agent::config::ForwardPlan {
    agent_forward_plan_for("v0.sma")
}

/// [`agent_forward_plan`] with the strategy id varied — distinct ids are what
/// let an order assertion tell two relayed plans apart.
fn agent_forward_plan_for(strategy: &str) -> agent::config::ForwardPlan {
    use trading_core::{Money, Price, Quantity, StrategyId, Symbol, Timestamp, Usdt};
    agent::config::ForwardPlan {
        strategy: StrategyId::new(strategy),
        symbol: Symbol::new("BTCUSDT"),
        stance: agent::config::PlanStance::Long,
        latest_signal: Some(agent::config::PlanSignal::Buy),
        rule: agent::config::PlanRuleKind::SmaCross {
            fast_len: 20,
            slow_len: 50,
        },
        last_close: Price::new(dec!(40000.00)).unwrap(),
        last_bar_ts: Timestamp::now(),
        budget: Money::<Usdt>::from_decimal(dec!(200)),
        projected_units: Quantity::new(dec!(0.005)).unwrap(),
        sizing_capped: false,
        horizon_days: 7,
        confidence: None, // P0-3: no scorecard in integration tests
    }
}

/// The iced runtime event stream a `Recipe::stream` receives; the take-once
/// recipes ignore it, so an empty one is faithful.
fn empty_event_stream() -> EventStream {
    Box::pin(futures::stream::empty::<iced::advanced::subscription::Event>())
}

/// Await the next item, FAILING the test if nothing resolves in `secs`.
///
/// The returned `Option` is the stream's own verdict: `Some(msg)` = a relayed
/// message, `None` = the stream ENDED. A timeout is never read as either
/// (the accepting `Err(_) => break` shape is exactly the bug-log #66 vacuity).
async fn next_or_fail(
    stream: &mut BoxStream<'static, Message>,
    secs: u64,
    what: &str,
) -> Option<Message> {
    timeout(Duration::from_secs(secs), stream.next())
        .await
        .unwrap_or_else(|_| panic!("timed out after {secs}s waiting for {what}"))
}

/// Assert the stream is LIVE with NOTHING buffered: a single poll returns
/// `Pending` (the relay is parked on its receiver). An observation, not a
/// count: it fails if the stream already ENDED (an empty or stopped relay) and
/// if it yields an item that was never sent.
fn assert_pending(stream: &mut BoxStream<'static, Message>, what: &str) {
    match stream.next().now_or_never() {
        None => {}
        Some(None) => panic!("{what}: the stream already ENDED (expected it live on its receiver)"),
        Some(Some(msg)) => panic!("{what}: an unexpected extra message was buffered: {msg:?}"),
    }
}

/// Drain a stream whose senders are already dropped: every item until the
/// stream END (`Ok(None)`), each step under a failing timeout. Bounded so a
/// runaway stream fails instead of spinning.
async fn drain_to_end(stream: &mut BoxStream<'static, Message>, what: &str) -> Vec<Message> {
    let mut out = Vec::new();
    loop {
        match next_or_fail(stream, 2, what).await {
            Some(msg) => out.push(msg),
            None => return out,
        }
        assert!(out.len() <= 64, "{what}: stream did not end after 64 items");
    }
}

/// The relayed plan view, or a panic naming what arrived instead.
fn plan_view(msg: &Message) -> &ui::forward_plan::ForwardPlanView {
    match msg {
        Message::ForwardPlanReceived(view) => view,
        other => panic!("expected ForwardPlanReceived, got {other:?}"),
    }
}

/// A cockpit with a structured bake-off result on screen and narration not yet
/// requested — the state the "Explain" guard requires. Driven through `update`
/// exactly as the binary drives it.
///
/// A PRIOR run whose narration resolved is played first, so the
/// `NotRequested` precondition can actually fail: from a bare `Cockpit::new()`
/// it held by `Default` alone. Here it holds only because
/// `BakeoffRunRequested` → `begin_run` reset the prior run's `Ready` (T-T4 P18).
fn cockpit_with_ready_bakeoff() -> Cockpit {
    const STALE: &str = "A PRIOR run's narration — must not carry into the next run.";
    let mut cockpit = Cockpit::new();

    // Prior run: crowned, explained, resolved.
    update(&mut cockpit, Message::BakeoffRunRequested);
    update(
        &mut cockpit,
        Message::BakeoffRunCompleted(Ok(ui::fixtures::fake_bakeoff_report_mirror())),
    );
    update(&mut cockpit, Message::BakeoffNarrationRequested);
    update(
        &mut cockpit,
        Message::BakeoffNarrationCompleted(ui::leaderboard::NarrationOutcome::Ready(SmolStr::new(
            STALE,
        ))),
    );
    assert_eq!(
        cockpit.leaderboard_screen_state.narration,
        NarrationState::Ready(SmolStr::new(STALE)),
        "seed: the prior run's narration resolved (the state the next run must clear)"
    );

    // This run.
    update(&mut cockpit, Message::BakeoffRunRequested);
    update(
        &mut cockpit,
        Message::BakeoffRunCompleted(Ok(ui::fixtures::fake_bakeoff_report_mirror())),
    );
    assert!(
        matches!(
            cockpit.leaderboard_screen_state.result,
            PanelState::Ready(_)
        ),
        "precondition: the bake-off result is Ready (the Explain guard's first half)"
    );
    assert_eq!(
        cockpit.leaderboard_screen_state.narration,
        NarrationState::NotRequested,
        "precondition: the new run reset the prior run's narration (begin_run) — Explain is offered again"
    );
    cockpit
}

/// Press "Explain" and PROVE it flipped `NotRequested → InFlight` — the
/// precondition the outcome then resolves (pinned, not assumed).
fn request_explain(cockpit: &mut Cockpit) {
    update(cockpit, Message::BakeoffNarrationRequested);
    assert_eq!(
        cockpit.leaderboard_screen_state.narration,
        NarrationState::InFlight,
        "precondition: BakeoffNarrationRequested flipped NotRequested → InFlight"
    );
}

// ── F6 — forward-plan relay ─────────────────────────────────────────────────

/// **F6 last-mile proof — the adapter at the seam.** Feeding an
/// `agent::ForwardPlan` into the plan channel makes the relay yield
/// `Message::ForwardPlanReceived` with the engine type mirrored field by field,
/// and applying that message via `update` lands EXACTLY that view on the Plan
/// screen. Then the stream ends on sender drop.
///
/// Its unique content is the field mirroring. It relays one plan, so it does
/// not guard the loop (T-T4 P1 survives here by design; the in-order test
/// below is that guard), and `set_plan` is unconditional, so no transition
/// precondition is asserted.
#[tokio::test]
async fn forward_plan_relay_yields_message_and_populates_state() {
    let (tx, rx) = tokio::sync::mpsc::channel::<agent::ForwardPlan>(4);
    let mut stream = ui::live::forward_plan_stream_impl(Some(rx));

    tx.send(agent_forward_plan()).await.unwrap();

    let msg = next_or_fail(&mut stream, 2, "the relayed plan")
        .await
        .expect("the relay yielded a Message");

    // 1) The relay maps the agent ForwardPlan → Message::ForwardPlanReceived
    //    with a faithfully-mirrored view (not the agent type — the pure-ui view).
    let view = plan_view(&msg).clone();
    assert_eq!(view.strategy, "v0.sma", "winner strategy id mirrored");
    assert_eq!(view.symbol, "BTCUSDT", "coin mirrored");
    assert_eq!(
        view.last_close,
        dec!(40000.00),
        "last close unwrapped to Decimal"
    );
    assert!(
        matches!(view.stance, ui::forward_plan::PlanStanceView::Long),
        "stance mirrored to the closed ui enum"
    );
    assert_eq!(view.budget, dec!(200), "budget unwrapped to Decimal");
    assert_eq!(
        view.projected_units,
        dec!(0.005),
        "projected units unwrapped to Decimal"
    );
    assert_eq!(view.horizon_days, 7, "horizon mirrored");

    // 2) Applying the Message lands exactly the relayed view on the Plan screen.
    let mut cockpit = Cockpit::new();
    update(&mut cockpit, msg);
    match &cockpit.forward_plan_screen_state.plan {
        PanelState::Ready(v) => assert_eq!(v, &view, "the Plan screen holds the relayed view"),
        other => panic!("expected the Plan state to become Ready, got {other:?}"),
    }

    // 3) The relay ends when the channel closes (T-T4 P14).
    drop(tx);
    let end = next_or_fail(&mut stream, 2, "the relay end after drop").await;
    assert!(end.is_none(), "the relay ends on sender drop (got {end:?})");
}

/// **Multi-item relay (T-T4 P1, the AC4-designated probe).** Two plans sent,
/// sender dropped: EXACTLY two `ForwardPlanReceived`, in send order, then the
/// stream ends. A `while`→`if` relay yields one and stops — red here, while
/// the single-plan test above stays green (which is why that test alone is not
/// a guard on the loop).
#[tokio::test]
async fn forward_plan_relay_yields_one_message_per_plan_in_order() {
    let (tx, rx) = tokio::sync::mpsc::channel::<agent::ForwardPlan>(4);
    let mut stream = ui::live::forward_plan_stream_impl(Some(rx));

    tx.send(agent_forward_plan_for("v0.sma")).await.unwrap();
    tx.send(agent_forward_plan_for("v0.5.rsi")).await.unwrap();
    drop(tx);

    let msgs = drain_to_end(&mut stream, "the two relayed plans, then the end").await;
    assert_eq!(
        msgs.len(),
        2,
        "exactly one ForwardPlanReceived per plan sent (got {msgs:?})"
    );
    let strategies: Vec<&str> = msgs
        .iter()
        .map(|m| plan_view(m).strategy.as_str())
        .collect();
    assert_eq!(
        strategies,
        ["v0.sma", "v0.5.rsi"],
        "plans relayed in send order"
    );
    for m in &msgs {
        assert_eq!(
            plan_view(m).symbol,
            "BTCUSDT",
            "coin mirrored on every plan"
        );
    }
}

/// The relay forwards what was buffered and THEN terminates when the sender
/// drops (channel closed) — no hang, no panic. Rewritten for story 2-63: the
/// old version sent nothing, so a relay that never enters its loop (T-T4 P2)
/// also "terminated" and passed. Now exactly one message must precede the end.
#[tokio::test]
async fn forward_plan_relay_terminates_on_sender_drop() {
    let (tx, rx) = tokio::sync::mpsc::channel::<agent::ForwardPlan>(4);
    let mut stream = ui::live::forward_plan_stream_impl(Some(rx));

    tx.send(agent_forward_plan()).await.unwrap();
    drop(tx);

    let first = next_or_fail(&mut stream, 2, "the buffered plan")
        .await
        .expect("the plan sent before the drop is relayed, not lost");
    assert_eq!(plan_view(&first).strategy, "v0.sma", "the buffered plan");

    let end = next_or_fail(&mut stream, 2, "the stream end after the drop").await;
    assert!(
        end.is_none(),
        "the relay ends when the channel closes (got {end:?})"
    );
}

/// `None` receiver (the double-`take()` case) yields nothing — the silent-empty
/// guard the `lab_progress_recipe_stream.rs` smoking-gun test pins.
#[tokio::test]
async fn forward_plan_relay_none_yields_nothing() {
    let mut stream = ui::live::forward_plan_stream_impl(None);
    let next = timeout(Duration::from_secs(1), stream.next())
        .await
        .expect("resolved within 1s");
    assert!(next.is_none(), "None receiver ⇒ empty stream");
}

/// **Recipe-path integration (T-T4 P3, P15).** The FULL
/// `<ForwardPlanRecipe as Recipe>::stream` — enter the runtime, `take()` the
/// receiver out of the `Arc<Mutex<Option<_>>>`, delegate to
/// `forward_plan_stream_impl` — the code `cockpit_live::subscription()` runs.
/// Before 2-63 this recipe had zero test references. Three separate claims:
///
/// 1. **take + own:** `stream()` empties the cell AND returns a stream that is
///    live on the receiver it took — `Pending`, not an already-ended empty
///    stream (P3 is red here, before any send) — and that relays a plan.
/// 2. **`stream()` on a taken cell ENDS:** a second recipe on the same Arc (a
///    re-stream without a fresh receiver) yields an empty stream that finishes
///    within 1 s. This is `stream()`'s own empty-cell path, which the
///    impl-level `forward_plan_relay_none_yields_nothing` bypasses (P15 is red
///    only here).
/// 3. **end:** the first stream ends on sender drop.
#[tokio::test]
async fn forward_plan_recipe_stream_takes_receiver_once_and_relays() {
    let (tx, rx) = tokio::sync::mpsc::channel::<agent::ForwardPlan>(4);
    let cell = Arc::new(Mutex::new(Some(rx)));

    let recipe = ForwardPlanRecipe {
        rt_handle: tokio::runtime::Handle::current(),
        rx: Arc::clone(&cell),
    };
    let mut stream = Box::new(recipe).stream(empty_event_stream());

    // 1) take + own.
    assert!(
        cell.lock().unwrap().is_none(),
        "Recipe::stream take()-s the receiver: the cell holds None afterwards"
    );
    assert_pending(
        &mut stream,
        "Recipe::stream relays the receiver it took (a live stream, not an empty one)",
    );
    assert!(
        !tx.is_closed(),
        "the receiver Recipe::stream took is alive inside the returned stream"
    );
    tx.send(agent_forward_plan_for("v0.5.rsi")).await.unwrap();
    let msg = next_or_fail(&mut stream, 2, "the plan via Recipe::stream")
        .await
        .expect("the plan sent after stream() is relayed by the recipe's stream");
    let view = plan_view(&msg);
    assert_eq!(
        view.strategy, "v0.5.rsi",
        "strategy carried through the recipe"
    );
    assert_eq!(view.symbol, "BTCUSDT", "symbol carried through the recipe");

    // 2) stream() on the now-taken cell ends instead of parking.
    let second = ForwardPlanRecipe {
        rt_handle: tokio::runtime::Handle::current(),
        rx: Arc::clone(&cell),
    };
    let mut second_stream = Box::new(second).stream(empty_event_stream());
    let none = next_or_fail(&mut second_stream, 1, "the second recipe's end").await;
    assert!(
        none.is_none(),
        "a second recipe on a taken cell is empty (got {none:?})"
    );

    // 3) end.
    drop(tx);
    let end = next_or_fail(&mut stream, 2, "the first recipe's end after drop").await;
    assert!(
        end.is_none(),
        "the recipe stream ends on sender drop (got {end:?})"
    );
}

// ── F9 — narration-outcome relay ────────────────────────────────────────────

/// **F9 outcome last-mile proof.** Feeding `agent::NarrationOutcome::Ready(prose)`
/// into the outcome channel makes the relay yield
/// `Message::BakeoffNarrationCompleted(Ready(_))`, AND applying it flips the
/// leaderboard `NarrationState` from a PROVEN `InFlight` to `Ready(prose)`
/// (the prose the operator sees). This is the return path that was dead.
#[tokio::test]
async fn narration_outcome_relay_ready_yields_message_and_sets_state() {
    let prose = "SMA crossover came out on top here, with the strongest \
                 risk-adjusted return across the field.";
    let (tx, rx) = tokio::sync::mpsc::channel::<agent::NarrationOutcome>(4);
    let mut stream = ui::live::narration_outcome_stream_impl(Some(rx));

    tx.send(agent::NarrationOutcome::Ready(SmolStr::new(prose)))
        .await
        .unwrap();

    let msg = timeout(Duration::from_secs(2), stream.next())
        .await
        .expect("a Message arrived within 2s")
        .expect("the relay yielded a Message");

    // 1) agent::NarrationOutcome → Message::BakeoffNarrationCompleted(ui outcome).
    match &msg {
        Message::BakeoffNarrationCompleted(ui::leaderboard::NarrationOutcome::Ready(p)) => {
            assert_eq!(p.as_str(), prose, "prose mirrored verbatim across the seam");
        }
        other => panic!("expected BakeoffNarrationCompleted(Ready), got {other:?}"),
    }

    // 2) Applying it resolves the in-flight narration to Ready (the prose card
    //    state). The InFlight precondition is driven through `update`, not
    //    assumed (T-T4 P6), from a seeded NotRequested (T-T4 P18).
    let mut cockpit = cockpit_with_ready_bakeoff();
    request_explain(&mut cockpit);
    update(&mut cockpit, msg);
    match &cockpit.leaderboard_screen_state.narration {
        NarrationState::Ready(p) => assert_eq!(p.as_str(), prose),
        other => panic!("expected NarrationState::Ready, got {other:?}"),
    }

    drop(tx);
    let end = next_or_fail(&mut stream, 2, "the relay end after drop").await;
    assert!(end.is_none(), "the relay ends on sender drop (got {end:?})");
}

/// **F9 fallback path.** `agent::NarrationOutcome::FellBack` relays to
/// `BakeoffNarrationCompleted(FellBack)` and resolves a PROVEN `InFlight`
/// narration to `NarrationState::FellBack` (the honest templated floor).
/// Proves the relay discriminates the two arms.
#[tokio::test]
async fn narration_outcome_relay_fellback_yields_message_and_sets_state() {
    let (tx, rx) = tokio::sync::mpsc::channel::<agent::NarrationOutcome>(4);
    let mut stream = ui::live::narration_outcome_stream_impl(Some(rx));

    tx.send(agent::NarrationOutcome::FellBack).await.unwrap();

    let msg = timeout(Duration::from_secs(2), stream.next())
        .await
        .expect("a Message arrived within 2s")
        .expect("the relay yielded a Message");

    assert!(
        matches!(
            &msg,
            Message::BakeoffNarrationCompleted(ui::leaderboard::NarrationOutcome::FellBack)
        ),
        "FellBack relays as FellBack, got {msg:?}"
    );

    let mut cockpit = cockpit_with_ready_bakeoff();
    request_explain(&mut cockpit);
    update(&mut cockpit, msg);
    assert!(
        matches!(
            cockpit.leaderboard_screen_state.narration,
            NarrationState::FellBack
        ),
        "FellBack lands the honest templated floor"
    );

    drop(tx);
    let end = next_or_fail(&mut stream, 2, "the relay end after drop").await;
    assert!(end.is_none(), "the relay ends on sender drop (got {end:?})");
}

/// **Two bake-off runs through ONE receiver (T-T4 P4, the AC4-designated
/// probe; P16).** Production builds the narration-outcome receiver once per
/// cockpit and streams it for the whole session, so the relay must keep
/// draining across runs. One stream for the whole test:
///
/// - Run 1: Requested → Completed(Ok) → Explain (`InFlight`) → relay `Ready(p1)`.
/// - Run 2: Requested resets the narration to `NotRequested` → Completed(Ok) →
///   Explain (`InFlight`) → relay `FellBack`.
///
/// Exactly ONE message per run is OBSERVED, not counted: after each run's
/// outcome, [`assert_pending`] proves the stream is still live (a `while`→`if`
/// relay has ENDED after run 1 — P4) and holds nothing else (an extra yield is
/// caught — P16). Then the end on sender drop.
#[tokio::test]
async fn narration_outcome_stream_resolves_two_bakeoff_runs_through_one_receiver() {
    const P1: &str = "Run one: the SMA crossover led on risk-adjusted return.";
    let (tx, rx) = tokio::sync::mpsc::channel::<agent::NarrationOutcome>(4);
    let mut stream = ui::live::narration_outcome_stream_impl(Some(rx));

    // ── Run 1 ──
    let mut cockpit = cockpit_with_ready_bakeoff();
    request_explain(&mut cockpit);
    tx.send(agent::NarrationOutcome::Ready(SmolStr::new(P1)))
        .await
        .unwrap();
    let msg = next_or_fail(&mut stream, 2, "run 1's narration outcome")
        .await
        .expect("run 1: the relay yielded an outcome");
    assert!(
        matches!(
            &msg,
            Message::BakeoffNarrationCompleted(ui::leaderboard::NarrationOutcome::Ready(p))
                if p.as_str() == P1
        ),
        "run 1 relays Ready(p1), got {msg:?}"
    );
    assert_pending(
        &mut stream,
        "after run 1's outcome (exactly one message for run 1; the relay must stay live for run 2)",
    );
    update(&mut cockpit, msg);
    assert_eq!(
        cockpit.leaderboard_screen_state.narration,
        NarrationState::Ready(SmolStr::new(P1)),
        "run 1 resolves to Ready(p1)"
    );

    // ── Run 2 (same cockpit, same stream) ──
    update(&mut cockpit, Message::BakeoffRunRequested);
    assert_eq!(
        cockpit.leaderboard_screen_state.narration,
        NarrationState::NotRequested,
        "a fresh bake-off resets the prior run's narration (begin_run)"
    );
    update(
        &mut cockpit,
        Message::BakeoffRunCompleted(Ok(ui::fixtures::fake_bakeoff_report_mirror())),
    );
    assert!(
        matches!(
            cockpit.leaderboard_screen_state.result,
            PanelState::Ready(_)
        ),
        "run 2 result is Ready"
    );
    request_explain(&mut cockpit);
    tx.send(agent::NarrationOutcome::FellBack).await.unwrap();
    let msg = next_or_fail(&mut stream, 2, "run 2's narration outcome")
        .await
        .expect("run 2: the SAME receiver relayed a second outcome (not an ended stream)");
    assert!(
        matches!(
            &msg,
            Message::BakeoffNarrationCompleted(ui::leaderboard::NarrationOutcome::FellBack)
        ),
        "run 2 relays FellBack, got {msg:?}"
    );
    assert_pending(
        &mut stream,
        "after run 2's outcome (exactly one message for run 2)",
    );
    update(&mut cockpit, msg);
    assert_eq!(
        cockpit.leaderboard_screen_state.narration,
        NarrationState::FellBack,
        "run 2 resolves to FellBack"
    );

    drop(tx);
    let end = next_or_fail(&mut stream, 2, "the relay end after both runs").await;
    assert!(
        end.is_none(),
        "the relay ends on sender drop after both runs (got {end:?})"
    );
}

/// **Recipe-path integration (T-T4 P5, P17).** The FULL
/// `<NarrationOutcomeRecipe as Recipe>::stream` — the code
/// `cockpit_live::subscription()` runs; zero test references before 2-63.
/// Same three claims as the forward-plan recipe test: `stream()` empties the
/// cell and returns a stream live on the receiver it took (P5 red at
/// `assert_pending`, before any send) that relays `Ready("p")`; `stream()` on
/// the taken cell ends instead of parking (P17 red only here); the first
/// stream ends on sender drop.
#[tokio::test]
async fn narration_outcome_recipe_stream_takes_receiver_once_and_relays() {
    let (tx, rx) = tokio::sync::mpsc::channel::<agent::NarrationOutcome>(4);
    let cell = Arc::new(Mutex::new(Some(rx)));

    let recipe = NarrationOutcomeRecipe {
        rt_handle: tokio::runtime::Handle::current(),
        rx: Arc::clone(&cell),
    };
    let mut stream = Box::new(recipe).stream(empty_event_stream());

    // 1) take + own.
    assert!(
        cell.lock().unwrap().is_none(),
        "Recipe::stream take()-s the receiver: the cell holds None afterwards"
    );
    assert_pending(
        &mut stream,
        "Recipe::stream relays the receiver it took (a live stream, not an empty one)",
    );
    assert!(
        !tx.is_closed(),
        "the receiver Recipe::stream took is alive inside the returned stream"
    );
    tx.send(agent::NarrationOutcome::Ready(SmolStr::new("p")))
        .await
        .unwrap();
    let msg = next_or_fail(&mut stream, 2, "the outcome via Recipe::stream")
        .await
        .expect("the outcome sent after stream() is relayed by the recipe's stream");
    assert!(
        matches!(
            &msg,
            Message::BakeoffNarrationCompleted(ui::leaderboard::NarrationOutcome::Ready(p))
                if p.as_str() == "p"
        ),
        "Ready(\"p\") relayed through the recipe, got {msg:?}"
    );

    // 2) stream() on the now-taken cell ends instead of parking.
    let second = NarrationOutcomeRecipe {
        rt_handle: tokio::runtime::Handle::current(),
        rx: Arc::clone(&cell),
    };
    let mut second_stream = Box::new(second).stream(empty_event_stream());
    let none = next_or_fail(&mut second_stream, 1, "the second recipe's end").await;
    assert!(
        none.is_none(),
        "a second recipe on a taken cell is empty (got {none:?})"
    );

    // 3) end.
    drop(tx);
    let end = next_or_fail(&mut stream, 2, "the first recipe's end after drop").await;
    assert!(
        end.is_none(),
        "the recipe stream ends on sender drop (got {end:?})"
    );
}

// ── F9 — Explain request BUILD (iced→agent) ─────────────────────────────────

/// Assert one mirror's request carries that mirror's facts. Every expected
/// value is DERIVED from the mirror, never copied from the subject's output.
fn assert_request_mirrors(mirror: &BakeoffReportMirror, label: &str) {
    let facts = ui::live::narration_request_from_mirror(mirror).facts;
    let rec = &mirror.recommendation;

    assert_eq!(
        facts.winner_id, rec.winner,
        "{label}: the crowned winner id is carried"
    );
    let want_ids: Vec<SmolStr> = mirror.rows.iter().map(|r| r.strategy.clone()).collect();
    assert_eq!(
        facts.candidate_ids, want_ids,
        "{label}: every candidate id is carried, in order"
    );
    assert_eq!(
        facts.candidate_kpi_strings.len(),
        mirror.rows.len(),
        "{label}: one KPI-string set per candidate"
    );
    for (kpi, row) in facts.candidate_kpi_strings.iter().zip(&mirror.rows) {
        assert_eq!(
            kpi.strategy_id, row.strategy,
            "{label}: KPI strings keyed to their own row"
        );
        // The Sharpe string is the canonical 4-dp form `agent::narration` expects.
        assert!(
            kpi.sharpe.contains('.') && kpi.sharpe.split('.').nth(1).unwrap().len() == 4,
            "{label}: Sharpe of {} is the canonical 4-dp string (got {:?})",
            row.strategy,
            kpi.sharpe
        );
    }
    let outcome_carried = match rec.outcome {
        OutcomeKind::ActiveWins => matches!(
            facts.outcome,
            agent::narration::NarrationOutcome_::ActiveWins
        ),
        OutcomeKind::BenchmarkWins => matches!(
            facts.outcome,
            agent::narration::NarrationOutcome_::BenchmarkWins
        ),
        OutcomeKind::AllFragile => matches!(
            facts.outcome,
            agent::narration::NarrationOutcome_::AllFragile
        ),
    };
    assert!(
        outcome_carried,
        "{label}: the mirror's {:?} outcome is carried (got {:?})",
        rec.outcome, facts.outcome
    );
    assert_eq!(
        facts.winner_robustness_label.is_some(),
        rec.winner_robustness.is_some(),
        "{label}: a robustness label is carried iff the mirror has one"
    );
    assert_eq!(
        facts.reason_codes.len(),
        rec.reasons.len(),
        "{label}: one reason code per mirror reason"
    );
}

/// **F9 request BUILD, two-sided (T-T4 P19).** `narration_request_from_mirror`
/// — the build the binary's Explain dispatch `try_send`s — carries the
/// on-screen mirror's facts: winner, every candidate id in order, the KPI
/// strings keyed per row, the outcome, the robustness label, the reasons.
///
/// Two fixtures that DIFFER in each of those dimensions, asserted below, so a
/// constant or positional mapping cannot satisfy both. The single-fixture
/// version this replaces also routed the request through an mpsc channel the
/// test built and drained itself — a self-loop that tested tokio, not the
/// cockpit (the enqueue is binary-side, see the module doc).
#[test]
fn narration_request_from_mirror_carries_the_on_screen_facts() {
    let active = ui::fixtures::fake_bakeoff_report_mirror();
    let bench = ui::fixtures::fake_bakeoff_report_mirror_benchmark_wins();

    // The pair pins a dimension only if it differs in it.
    assert_ne!(
        active.recommendation.outcome, bench.recommendation.outcome,
        "fixture pair: the outcomes differ"
    );
    assert_ne!(
        bench.recommendation.winner, bench.rows[0].strategy,
        "fixture pair: the BenchmarkWins crown is NOT row 0 (a positional winner fails it)"
    );
    assert_eq!(
        active.recommendation.winner, active.rows[0].strategy,
        "fixture pair: the ActiveWins crown IS row 0 (the other side of that dimension)"
    );
    assert_ne!(
        active.rows.len(),
        bench.rows.len(),
        "fixture pair: the candidate counts differ"
    );
    assert_ne!(
        active.recommendation.winner_robustness.is_some(),
        bench.recommendation.winner_robustness.is_some(),
        "fixture pair: one carries a robustness label, the other does not"
    );
    assert_ne!(
        active.recommendation.reasons.len(),
        bench.recommendation.reasons.len(),
        "fixture pair: the reason counts differ"
    );

    assert_request_mirrors(&active, "ActiveWins mirror");
    assert_request_mirrors(&bench, "BenchmarkWins mirror");
}

/// **F9 Sortino/Calmar non-empty proof.** The `narration_request_from_mirror`
/// builder must populate non-empty sortino and calmar strings in every
/// `CandidateKpiStrings` (from the new `LeaderRow.sortino`/`.calmar` mirror
/// fields). Before the fix these were always `String::new()` — absent from the
/// P3 allowed-set — so any LLM Sortino/Calmar citation read as FABRICATED and
/// the narration fell back. This test pins that the strings are now non-empty
/// and in the canonical 4-dp form the post-check accepts.
#[tokio::test]
async fn explain_request_carries_non_empty_sortino_and_calmar() {
    let mirror = ui::fixtures::fake_bakeoff_report_mirror();
    let request = ui::live::narration_request_from_mirror(&mirror);
    let facts = &request.facts;

    // The per-candidate loop below is vacuous over an empty set (bug-log #66):
    // prove it iterates at least once.
    assert!(!facts.candidate_kpi_strings.is_empty());

    // Every row must have non-empty sortino and calmar strings.
    for (i, kpi) in facts.candidate_kpi_strings.iter().enumerate() {
        assert!(
            !kpi.sortino.is_empty(),
            "candidate[{i}] sortino must be non-empty — was always empty before the fix \
             (caused FellBack for any LLM Sortino citation)"
        );
        assert!(
            !kpi.calmar.is_empty(),
            "candidate[{i}] calmar must be non-empty — was always empty before the fix \
             (caused FellBack for any LLM Calmar citation)"
        );
        // The strings must be in the canonical 4-dp form (the P3 exact-match contract).
        assert!(
            kpi.sortino.contains('.') && kpi.sortino.split('.').nth(1).unwrap().len() == 4,
            "candidate[{i}] sortino must be canonical 4-dp (got {:?})",
            kpi.sortino
        );
        assert!(
            kpi.calmar.contains('.') && kpi.calmar.split('.').nth(1).unwrap().len() == 4,
            "candidate[{i}] calmar must be canonical 4-dp (got {:?})",
            kpi.calmar
        );
    }

    // The allowed-number set must include the winner's sortino and calmar —
    // the load-bearing check that those values are now accepted by P3.
    let allowed = facts.allowed_numbers();
    let winner_kpi = facts
        .candidate_kpi_strings
        .iter()
        .find(|k| k.strategy_id == facts.winner_id)
        .expect("winner KPI must be present");
    assert!(
        allowed.contains(&winner_kpi.sortino),
        "winner sortino {:?} must be in the P3 allowed set — before the fix it was \
         always empty and therefore always absent",
        winner_kpi.sortino
    );
    assert!(
        allowed.contains(&winner_kpi.calmar),
        "winner calmar {:?} must be in the P3 allowed set",
        winner_kpi.calmar
    );
}
