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
//! path end to end. They also assert the iced→agent "Explain" request-build
//! (`narration_request_from_mirror`) actually enqueues a faithful request.
//!
//! Gated on `live` (the only build where the wiring + the `agent` types exist),
//! NOT on `target_os` (no pixels here — pure channel/state logic, deterministic
//! on every OS).

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;

use futures::StreamExt;
use rust_decimal_macros::dec;
use smol_str::SmolStr;
use tokio::time::timeout;

use ui::leaderboard::NarrationState;
use ui::state::{Cockpit, Message, PanelState, update};

// ── Builders ──────────────────────────────────────────────────────────────────

/// A populated, deterministic `agent::config::ForwardPlan` (the agent-side type
/// the supervisor sends over `plan_tx`). SMA-cross, LONG, BTCUSDT — mirrors the
/// `fake_forward_plan` fixture's shape but as the AGENT type, so the relay
/// genuinely exercises the `ForwardPlanView::from_plan` adapter at the boundary.
fn agent_forward_plan() -> agent::config::ForwardPlan {
    use trading_core::{Money, Price, Quantity, StrategyId, Symbol, Timestamp, Usdt};
    agent::config::ForwardPlan {
        strategy: StrategyId::new("v0.sma"),
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

// ── F6 — forward-plan relay ─────────────────────────────────────────────────

/// **F6 last-mile proof.** Feeding an `agent::ForwardPlan` into the plan channel
/// makes the relay yield `Message::ForwardPlanReceived`, AND applying that
/// message via `update` lands the Plan screen state at the POPULATED
/// `Ready(view)` (the engine type mirrored faithfully across the seam). This is
/// the exact path that was dead — the receiver was never consumed.
#[tokio::test]
async fn forward_plan_relay_yields_message_and_populates_state() {
    let (tx, rx) = tokio::sync::mpsc::channel::<agent::ForwardPlan>(4);
    let mut stream = ui::live::forward_plan_stream_impl(Some(rx));

    tx.send(agent_forward_plan()).await.unwrap();

    let msg = timeout(Duration::from_secs(2), stream.next())
        .await
        .expect("a Message arrived within 2s")
        .expect("the relay yielded a Message");

    // 1) The relay maps the agent ForwardPlan → Message::ForwardPlanReceived
    //    with a faithfully-mirrored view (not the agent type — the pure-ui view).
    let view = match &msg {
        Message::ForwardPlanReceived(view) => view.clone(),
        other => panic!("expected ForwardPlanReceived, got {other:?}"),
    };
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

    // 2) Applying the Message populates the F6 Plan screen state (Empty → Ready).
    let mut cockpit = Cockpit::new();
    assert!(
        matches!(cockpit.forward_plan_screen_state.plan, PanelState::Empty),
        "precondition: the Plan screen starts Empty (the live bug's symptom)"
    );
    update(&mut cockpit, msg);
    match &cockpit.forward_plan_screen_state.plan {
        PanelState::Ready(v) => {
            assert_eq!(v.strategy, "v0.sma");
            assert_eq!(v.symbol, "BTCUSDT");
        }
        other => panic!("expected the Plan state to become Ready, got {other:?}"),
    }
}

/// The relay terminates cleanly when the sender drops (channel closed) — no
/// hang, no panic. Mirrors the `LabProgressRecipe` close behaviour.
#[tokio::test]
async fn forward_plan_relay_terminates_on_sender_drop() {
    let (tx, rx) = tokio::sync::mpsc::channel::<agent::ForwardPlan>(4);
    let mut stream = ui::live::forward_plan_stream_impl(Some(rx));
    drop(tx);
    let next = timeout(Duration::from_secs(2), stream.next())
        .await
        .expect("the stream resolved within 2s");
    assert!(next.is_none(), "the relay ends when the channel closes");
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

// ── F9 — narration-outcome relay ────────────────────────────────────────────

/// **F9 outcome last-mile proof.** Feeding `agent::NarrationOutcome::Ready(prose)`
/// into the outcome channel makes the relay yield
/// `Message::BakeoffNarrationCompleted(Ready(_))`, AND applying it flips the
/// leaderboard `NarrationState` to `Ready(prose)` (the prose the operator sees).
/// This is the return path that was dead.
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

    // 2) Applying it sets the NarrationState to Ready (the prose card state).
    let mut cockpit = Cockpit::new();
    update(&mut cockpit, msg);
    match &cockpit.leaderboard_screen_state.narration {
        NarrationState::Ready(p) => assert_eq!(p.as_str(), prose),
        other => panic!("expected NarrationState::Ready, got {other:?}"),
    }
}

/// **F9 fallback path.** `agent::NarrationOutcome::FellBack` relays to
/// `BakeoffNarrationCompleted(FellBack)` and lands `NarrationState::FellBack`
/// (the honest templated floor). Proves the relay discriminates the two arms.
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

    let mut cockpit = Cockpit::new();
    update(&mut cockpit, msg);
    assert!(
        matches!(
            cockpit.leaderboard_screen_state.narration,
            NarrationState::FellBack
        ),
        "FellBack lands the honest templated floor"
    );
}

// ── F9 — Explain request (iced→agent) ───────────────────────────────────────

/// **F9 request last-mile proof.** The "Explain" action must ENQUEUE a
/// `NarrationRequest` on the request channel carrying the on-screen mirror's
/// facts — NOT a bare `FellBack`. This drives the exact `narration_request_from_mirror`
/// build the cockpit's `BakeoffNarrationRequested` arm `try_send`s, and asserts
/// the request lands on the channel with faithful prompt facts (winner, all
/// candidate ids, the canonical KPI strings for the KPIs the mirror carries).
#[tokio::test]
async fn explain_action_enqueues_request_with_faithful_facts() {
    let mirror = ui::fixtures::fake_bakeoff_report_mirror();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<agent::narration::NarrationRequest>(4);

    // The cockpit's BakeoffNarrationRequested arm builds the request from the
    // on-screen mirror and try_sends it — exercise that exact build + send.
    let request = ui::live::narration_request_from_mirror(&mirror);
    tx.try_send(request)
        .expect("request enqueued on the channel");

    let received = timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("recv resolved within 1s")
        .expect("a request arrived on the channel — NOT a bare FellBack");
    let facts = &received.facts;

    // The winner id + every candidate id cross faithfully.
    assert_eq!(
        facts.winner_id, mirror.recommendation.winner,
        "the crowned winner id is carried"
    );
    let want_ids: Vec<SmolStr> = mirror.rows.iter().map(|r| r.strategy.clone()).collect();
    assert_eq!(
        facts.candidate_ids, want_ids,
        "every candidate id is carried, in order"
    );
    assert_eq!(
        facts.candidate_kpi_strings.len(),
        mirror.rows.len(),
        "one KPI-string set per candidate"
    );
    // The Sharpe string is the canonical 4-dp form `agent::narration` expects.
    let first = &facts.candidate_kpi_strings[0];
    assert_eq!(
        first.strategy_id, mirror.rows[0].strategy,
        "KPI strings keyed to the right strategy"
    );
    assert!(
        first.sharpe.contains('.') && first.sharpe.split('.').nth(1).unwrap().len() == 4,
        "Sharpe is the canonical 4-dp string (got {:?})",
        first.sharpe
    );
    // The outcome enum maps faithfully (ActiveWins for the fixture).
    assert!(
        matches!(
            facts.outcome,
            agent::narration::NarrationOutcome_::ActiveWins
        ),
        "the fixture mirror's ActiveWins outcome is carried"
    );
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
